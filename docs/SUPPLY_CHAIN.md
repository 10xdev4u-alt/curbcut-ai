# Supply chain: SBOM, provenance, and reproducible builds

CC-119 asks for a published SBOM, a provenance attestation per artifact, and
reproducible release builds. This page documents what the repository actually
does, and names the criteria it does not meet. The SBOM, the attestation
placeholder, and the release prerequisites are implemented and checked in CI;
the two-build comparison is not, and is tracked as #208.

## Release recipe

The recipe is deliberately boring so that another machine can repeat it:

```bash
rustup default stable          # the MSRV is recorded in the workspace manifest
cargo build --release --locked  # never let the resolver pick new versions
```

Reproducibility inputs the repository pins or checks:

- `Cargo.lock` is committed, and CI builds against that exact resolution.
- `Cargo.toml` declares `rust-version`, `edition`, and the workspace license.
- `[lints.rust]` forbids `unsafe_code`.
- `scripts/sbom.py` writes no timestamp and no random serial number, so two runs
  over the same tree produce byte-identical output. Its self-test asserts that.

What is *not* pinned: the exact compiler patch version. The recipe records it at
release time rather than forcing a toolchain download on every contributor.

## SBOM

`scripts/sbom.py generate` reads `Cargo.lock` for the resolved package set and
each crate's `Cargo.toml` for its license, and writes a CycloneDX-shaped
document. Every dependency edge is a full `bom-ref`, so the published graph
resolves to components in the same document; the workspace root appears once, in
`metadata.component`. Today the workspace is standard-library only, so every
component is a first-party crate under `AGPL-3.0-only`.

A dependency that lives outside the workspace has no manifest here. Its license
must be recorded in `supply-chain/registry-licenses.json`, keyed by `name@version`
(or `name`), with a known SPDX id. A package with no resolvable license is
recorded as `UNKNOWN` rather than guessed, and `--strict` fails the build until
someone declares it; the generator never invents a license.
`scripts/sbom.py check` re-derives the expected document and compares the root
component, every component's type, version, license, and every dependency edge.

CI publishes the generated SBOM as a build artifact on every run.

## Provenance attestation

`scripts/sbom.py generate --attestation` writes a placeholder attestation
carrying the target triple, the source revision, and empty `sha256` and
`signature` fields. It states in the document itself that it is an unsigned
placeholder. Filling those fields is a release-job step performed with a
maintainer key; this repository cannot create or verify a real signature, so no
artifact here is claimed to be signed.

## Two-build measurement (one machine)

Two isolated release builds of revision
`2c771b94dc363c028b9846bf785e04d7323bd431` were recorded on 2026-11-25 with
`cargo build --release --locked -p curbcut-cli`:

| Build | Source path | Target directory | Bytes | SHA-256 |
| --- | --- | --- | --- | --- |
| A | `/tmp/opencode/repro-a` | `/tmp/opencode/repro-a-target` | 376816 | `d7b9c351b03a1cc3b3c33c8bb526820ce99f68301ea7c397c33db59341bbeec1` |
| B | `/tmp/opencode/repro-b` | `/tmp/opencode/repro-b-target` | 376816 | `d7b9c351b03a1cc3b3c33c8bb526820ce99f68301ea7c397c33db59341bbeec1` |

Environment for both: Linux 7.2.8-9-omarchy x86_64, rustc 1.99.1
(`58cf8f9c35b355199727627d07ae34a4e2846d67`, 2026-10-13), host target
`x86_64-unknown-linux-gnu`. The raw record is in
`supply-chain/reproducibility.json`.

The digests match, so the recipe is path-independent on this machine with this
toolchain. That is **not** the two-independent-environments comparison CC-119
asks for: the kernel, compiler patch version, and CPU were the same. Only the
source and target paths differed. The exact compiler patch is still not pinned
by the recipe. #208 stays OPEN until a second machine or CI runner records the
same revision and publishes its digest.

## What is not proven

- **Two independent builds have not been shown to match by hash.** Byte-identical
  builds across machines depend on the exact compiler, target, and environment;
  the recipe above fixes inputs but no two-machine comparison has been recorded.
  Tracked as #208.
- **No signed attestation has been published.** The placeholder format exists;
  signing infrastructure does not.
- **Third-party license coverage is exercised only against a synthetic fixture**,
  because there are no third-party dependencies yet. The declaration file stays
  empty until such a dependency appears.

These are engineering gaps, not compliance claims. Nothing in this repository
should be read as asserting a reproducible hash or a signed release.
