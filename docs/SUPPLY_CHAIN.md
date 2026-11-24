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
document. Today the workspace is standard-library only, so every component is a
first-party crate under `AGPL-3.0-only`. The generator is written to handle
third-party dependencies too: a package with no resolvable license is recorded
as `UNKNOWN` rather than guessed, and `--strict` fails the build until someone
declares it. `scripts/sbom.py check` re-derives the expected document and
compares components, versions, licenses, and dependency edges.

CI publishes the generated SBOM as a build artifact on every run.

## Provenance attestation

`scripts/sbom.py generate --attestation` writes a placeholder attestation
carrying the target triple, the source revision, and empty `sha256` and
`signature` fields. It states in the document itself that it is an unsigned
placeholder. Filling those fields is a release-job step performed with a
maintainer key; this repository cannot create or verify a real signature, so no
artifact here is claimed to be signed.

## What is not proven

- **Two independent builds have not been shown to match by hash.** Byte-identical
  builds across machines depend on the exact compiler, target, and environment;
  the recipe above fixes inputs but no two-machine comparison has been recorded.
  Tracked as #208.
- **No signed attestation has been published.** The placeholder format exists;
  signing infrastructure does not.
- **Third-party license coverage is untested in practice**, because there are no
  third-party dependencies yet. The generator's self-test exercises that path
  against a synthetic manifest.

These are engineering gaps, not compliance claims. Nothing in this repository
should be read as asserting a reproducible hash or a signed release.
