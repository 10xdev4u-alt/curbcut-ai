#!/usr/bin/env python3
"""Fail if the update-check exception loses its basis, control, or honesty.

Issue #205 found a published opt-in promise that the code does not keep:
`UpdateCheck` is compiled into every build and denied by a default-engaged
internal kill switch. `docs/NETWORK_POLICY.md` records it as a deliberate
exception. A persisted root `update_check` key and a config-built dial path now
exist, but no shipped binary or UI calls them, so #221 stays open. This
validator checks that the document, its consumers, the findings register, the
code-level gates, the schema row, and the integration test stay consistent with
that statement.

Run ``--self-test`` to apply every named mutation to a pristine copy and
require detection. Every mutation source is preflighted, so a stale fixture
fails cleanly instead of silently passing.
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

POLICY = "docs/NETWORK_POLICY.md"
INVENTORY = "docs/DATA_INVENTORY.md"
DPIA = "docs/DPIA.md"
FINDINGS = "docs/PRIVACY_FINDINGS.md"
SECURITY = "SECURITY.md"
THREAT = "docs/THREAT_MODEL.md"
NET_RS = "crates/curbcut-core/src/net.rs"
CORE_README = "crates/curbcut-core/README.md"
COMMAND_RS = "crates/curbcut-cli/src/command.rs"
CONFIG_RS = "crates/curbcut-core/src/config.rs"
OPT_IN_TEST = "crates/curbcut-core/tests/network_opt_in.rs"
CLI_DOC = "docs/CLI.md"
CONFIG_SCHEMA = "docs/CONFIG_SCHEMA.md"
README = "README.md"
WORKFLOW = ".github/workflows/ci.yml"

DOCUMENTS = (
    POLICY,
    INVENTORY,
    DPIA,
    FINDINGS,
    SECURITY,
    THREAT,
    NET_RS,
    CORE_README,
    COMMAND_RS,
    CONFIG_RS,
    OPT_IN_TEST,
    CLI_DOC,
    CONFIG_SCHEMA,
    README,
    WORKFLOW,
)

CONSUMERS = (INVENTORY, DPIA, FINDINGS, SECURITY, THREAT)
CONSUMER_PHRASES = {
    INVENTORY: "documented exception in",
    DPIA: "one exception (f-12)",
    FINDINGS: "documented exception",
    SECURITY: "documented compiled-in update check",
    THREAT: "documented exception",
}

SECTIONS = ("The channels", "The modes", "The gate in use", "The update-check exception", "Limits")

REQUIRED_PHRASES = (
    "dc-30",
    "f-12",
    "r-09",
    "#205",
    "#221",
    "deliberate exception",
    "starts engaged",
    "no shipped binary calls",
    "no shipped binary releases a switch",
    "no ui to inspect or revoke",
    "persisted setting",
    "version only",
    "no screen content",
    "update::dial",
    "broken install",
    "stopped only by the runtime switch",
)

OVERCLAIMS = (
    "user has opted in",
    "user-facing opt-in exists",
    "per-user opt-in exists",
    "off by default until the user",
    "fully offline",
    "either one alone is enough",
    "either one is enough",
    "kill switch is a user control",
    "user opt-in is shipped",
    "wcag compliant",
    "ada compliant",
    "makes apps accessible",
)

CODE_INVARIANTS = (
    "pub mod update",
    "if feature != NetworkFeature::UpdateCheck",
    "if !switch.allows(feature)",
    "engaged: true",
    "matches!(self, Self::UpdateCheck)",
)

CONFIG_INVARIANTS = (
    "pub update_check: bool",
    "update_check: false,",
    '"update_check" => config.update_check = value.as_bool("update_check")?,',
)

FROM_CONFIG_INVARIANTS = ("NetworkPolicy::updates_only()",)

SCHEMA_ROW = "| `update_check` | boolean | `false` |"

TEST_NAMES = (
    "fn absent_update_check_key_denies_the_dial()",
    "fn explicit_false_denies_the_dial()",
    "fn true_allows_only_update_check_and_reaches_loopback()",
    "fn non_boolean_update_check_is_rejected()",
    "fn removing_the_key_revokes_the_choice()",
)

CLI_TOKENS = ("network-status", "network-revoke")

COMMAND_INVARIANTS = (
    "NetworkStatus,",
    "NetworkRevoke,",
    '"network-status" => Ok(Self::NetworkStatus)',
    '"network-revoke" => Ok(Self::NetworkRevoke)',
    "crate::network::network_status(input, out, err)",
    "crate::network::network_revoke(input, out, err)",
)


def flatten(text: str) -> str:
    return " ".join(text.lower().split())


def table_cells(text: str, first_cell: str) -> list[str] | None:
    for line in text.splitlines():
        stripped = line.strip()
        if not (stripped.startswith("|") and stripped.endswith("|")):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if cells and cells[0] == first_cell:
            return cells
    return None


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    documents: dict[str, str] = {}
    for relative in DOCUMENTS:
        path = root / relative
        if not path.is_file():
            errors.append(f"missing {relative}")
            continue
        try:
            documents[relative] = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            errors.append(f"{relative}: cannot read: {error}")
    if errors:
        return errors

    policy = documents[POLICY]
    for heading in SECTIONS:
        if not re.search(rf"^#{{1,6}}\s+{re.escape(heading)}\b", policy, re.MULTILINE):
            errors.append(f"NETWORK_POLICY.md is missing the {heading!r} section")
    body = flatten(policy)
    for phrase in REQUIRED_PHRASES:
        if phrase not in body:
            errors.append(f"NETWORK_POLICY.md no longer states {phrase!r}")
    for phrase in OVERCLAIMS:
        if phrase in body:
            errors.append(f"NETWORK_POLICY.md overclaims {phrase!r}")

    for relative in CONSUMERS:
        text = documents[relative]
        if "NETWORK_POLICY.md" not in text:
            errors.append(f"{relative} no longer references NETWORK_POLICY.md")
        phrase = CONSUMER_PHRASES[relative]
        if phrase not in flatten(text):
            errors.append(f"{relative} no longer states {phrase!r}")
        for phrase in OVERCLAIMS:
            if phrase in flatten(text):
                errors.append(f"{relative} overclaims {phrase!r}")

    dc30 = table_cells(documents[INVENTORY], "DC-30")
    if dc30 is None:
        errors.append("DATA_INVENTORY.md has no DC-30 row")
    elif len(dc30) < 6:
        errors.append("DC-30 row is truncated")
    elif dc30[5] != "legitimate-interests":
        errors.append(f"DC-30 basis is {dc30[5]!r}, not 'legitimate-interests'")

    f12 = table_cells(documents[FINDINGS], "F-12")
    if f12 is None:
        errors.append("PRIVACY_FINDINGS.md has no F-12 row")
    elif len(f12) < 3:
        errors.append("F-12 row is truncated")
    elif f12[2] != "MITIGATED":
        errors.append(f"F-12 status is {f12[2]!r}, not MITIGATED")

    r09 = table_cells(documents[DPIA], "R-09")
    if r09 is None:
        errors.append("DPIA.md has no R-09 row")
    elif len(r09) < 5:
        errors.append("R-09 row is truncated")
    elif "documented exception" not in flatten(r09[4]):
        errors.append("R-09 mitigation does not name the documented exception")
    elif "F-12" not in " ".join(r09):
        errors.append("R-09 no longer points at F-12")

    net = documents[NET_RS]
    if "no shipped binary calls this constructor" in net.lower():
        errors.append("net.rs still says no shipped binary calls from_config")
    core_readme = documents[CORE_README]
    if "no shipped binary calls that path yet" in core_readme.lower():
        errors.append("curbcut-core README still says no shipped binary calls from_config")
    for phrase in CODE_INVARIANTS + FROM_CONFIG_INVARIANTS:
        if phrase not in net:
            errors.append(f"net.rs no longer contains {phrase!r}")
    compact = re.sub(r"\s+", "", net)
    if "dial(&KillSwitch::from_config(config)" not in compact:
        errors.append("net.rs no longer delegates to dial(&KillSwitch::from_config")
    checks = net.count("if !switch.allows(feature)")
    if checks != 2:
        errors.append(f"net.rs has {checks} switch checks, expected 2")

    command = documents[COMMAND_RS]
    for phrase in COMMAND_INVARIANTS:
        if phrase not in command:
            errors.append(f"command.rs no longer contains {phrase!r}")
    for token in CLI_TOKENS:
        if token not in command:
            errors.append(f"command.rs no longer has the {token!r} command")
        if token not in policy:
            errors.append(f"NETWORK_POLICY.md no longer documents the {token!r} recipe")
        if token not in documents[CLI_DOC]:
            errors.append(f"CLI.md no longer documents the {token!r} command")

    config_rs = documents[CONFIG_RS]
    for phrase in CONFIG_INVARIANTS:
        if phrase not in config_rs:
            errors.append(f"config.rs no longer contains {phrase!r}")

    if SCHEMA_ROW not in documents[CONFIG_SCHEMA]:
        errors.append("CONFIG_SCHEMA.md has no update_check root-key row")

    opt_in_test = documents[OPT_IN_TEST]
    for name in TEST_NAMES:
        if name not in opt_in_test:
            errors.append(f"network_opt_in.rs no longer contains {name!r}")

    readme = flatten(documents[README])
    if "network_policy.md" not in readme:
        errors.append("README.md no longer indexes docs/NETWORK_POLICY.md")
    if "validate_network_policy.py" not in readme:
        errors.append("README.md no longer indexes scripts/validate_network_policy.py")
    workflow = documents[WORKFLOW]
    if "scripts/validate_network_policy.py --self-test" not in workflow:
        errors.append("CI no longer self-tests the network-policy validator")
    if not re.search(r"python3 scripts/validate_network_policy\.py\n", workflow):
        errors.append("CI no longer runs the network-policy validator")
    return errors


def collected_files(root: Path) -> list[str]:
    return list(DOCUMENTS)


def materialize(target: Path, pristine: dict[str, str]) -> None:
    if target.exists():
        shutil.rmtree(target)
    for relative, text in pristine.items():
        path = target / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8", newline="\n")


CASES: tuple[dict[str, object], ...] = (
    {
        "label": "canonical document deleted",
        "delete": [POLICY],
        "expect": "missing docs/NETWORK_POLICY.md",
    },
    {
        "label": "exception section renamed",
        "edit": [(POLICY, "## The update-check exception", "## Exception")],
        "expect": "'The update-check exception' section",
    },
    {
        "label": "opt-in invented",
        "replace": [(POLICY, "no shipped binary calls", "a shipped binary calls")],
        "expect": "'no shipped binary calls'",
    },
    {
        "label": "user consent overclaim",
        "edit": [
            (
                POLICY,
                "there is **no UI to inspect or revoke**",
                "The user has opted in to updates",
            )
        ],
        "expect": "overclaims 'user has opted in'",
    },
    {
        "label": "switch default inverted",
        "replace": [(POLICY, "starts engaged", "starts disengaged")],
        "expect": "'starts engaged'",
    },
    {
        "label": "shipped release caller invented",
        "replace": [
            (
                POLICY,
                "No shipped binary releases a switch it keeps or uses to dial",
                "A shipped binary releases a switch",
            )
        ],
        "expect": "'no shipped binary releases a switch'",
    },
    {
        "label": "issue reference changed",
        "replace": [(POLICY, "#205", "#999")],
        "expect": "'#205'",
    },
    {
        "label": "DC-30 basis changed",
        "edit": [
            (
                INVENTORY,
                "| RAM today; disk persistence planned | session | legitimate-interests",
                "| RAM today; disk persistence planned | session | contract",
            )
        ],
        "expect": "DC-30 basis",
    },
    {
        "label": "finding closed early",
        "edit": [(FINDINGS, "| F-12 | MEDIUM | MITIGATED |", "| F-12 | MEDIUM | OPEN |")],
        "expect": "F-12 status",
    },
    {
        "label": "R-09 loses the exception",
        "edit": [
            (
                DPIA,
                "documented exception in [`docs/NETWORK_POLICY.md`](NETWORK_POLICY.md); persisted `update_check`",
                "no user choice",
            )
        ],
        "expect": "R-09 mitigation does not name the documented exception",
    },
    {
        "label": "update dial loses the switch check",
        "replace": [(NET_RS, "if !switch.allows(feature)", "if false")],
        "expect": "net.rs no longer contains 'if !switch.allows(feature)'",
    },
    {
        "label": "update dial alone loses its switch check",
        "edit": [
            (
                NET_RS,
                "if feature != NetworkFeature::UpdateCheck {\n            return Err(NetworkError::Blocked);\n        }\n        if !switch.allows(feature) {",
                "if feature != NetworkFeature::UpdateCheck {\n            return Err(NetworkError::Blocked);\n        }\n        if true {",
            )
        ],
        "expect": "net.rs has 1 switch checks",
    },
    {
        "label": "config default flipped to true",
        "edit": [(CONFIG_RS, "update_check: false,", "update_check: true,")],
        "expect": "config.rs no longer contains 'update_check: false,'",
    },
    {
        "label": "config parse arm deleted",
        "edit": [
            (
                CONFIG_RS,
                '"update_check" => config.update_check = value.as_bool("update_check")?,',
                "",
            )
        ],
        "expect": "config.rs no longer contains '\"update_check\" =>",
    },
    {
        "label": "from_config widened to online",
        "edit": [
            (
                NET_RS,
                "let mut switch = Self::new(NetworkPolicy::updates_only());",
                "let mut switch = Self::new(NetworkPolicy::online());",
            )
        ],
        "expect": "net.rs no longer contains 'NetworkPolicy::updates_only()'",
    },
    {
        "label": "delegation removed",
        "replace": [(NET_RS, "KillSwitch::from_config(config)", "KillSwitch::engaged()")],
        "expect": "no longer delegates to dial(&KillSwitch::from_config",
    },
    {
        "label": "opt-in integration test deleted",
        "edit": [
            (
                OPT_IN_TEST,
                "fn true_allows_only_update_check_and_reaches_loopback()",
                "fn allows_update_check()",
            )
        ],
        "expect": "network_opt_in.rs no longer contains 'fn true_allows_only_update_check_and_reaches_loopback()'",
    },
    {
        "label": "config schema row removed",
        "edit": [(CONFIG_SCHEMA, SCHEMA_ROW, "| `update_check` | boolean | `true` |")],
        "expect": "CONFIG_SCHEMA.md has no update_check root-key row",
    },
    {
        "label": "security guide invents user consent",
        "edit": [(SECURITY, "Accessibility semantics are read-only by default", "The user has opted in to updates. Accessibility semantics are read-only by default")],
        "expect": "SECURITY.md overclaims 'user has opted in'",
    },
    {
        "label": "core constructor doc goes stale",
        "edit": [(NET_RS, "The process-local CLI calls this constructor to inspect the decision;", "No shipped binary calls this constructor.")],
        "expect": "net.rs still says no shipped binary calls from_config",
    },
    {
        "label": "core generated README goes stale",
        "edit": [(CORE_README, "a process-local CLI calls it to inspect the decision, but no shipped", "no shipped binary calls that path yet.")],
        "expect": "curbcut-core README still says",
    },
    {
        "label": "readme drops the validator",
        "edit": [(README, "validate_network_policy.py", "validate_removed.py")],
        "expect": "no longer indexes scripts/validate_network_policy.py",
    },
    {
        "label": "CI drops the self-test",
        "edit": [(WORKFLOW, "scripts/validate_network_policy.py --self-test", "scripts/validate_hashes.py --self-test")],
        "expect": "no longer self-tests the network-policy validator",
    },
    {
        "label": "CLI status token removed from the dispatcher",
        "replace": [(COMMAND_RS, "network-status", "netstat")],
        "expect": "command.rs no longer has the 'network-status' command",
    },
    {
        "label": "CLI recipe dropped from the command guide",
        "replace": [(CLI_DOC, "network-revoke", "revoke")],
        "expect": "CLI.md no longer documents the 'network-revoke' command",
    },
    {
        "label": "CLI recipe dropped from the network policy",
        "replace": [(POLICY, "network-status", "netstat")],
        "expect": "NETWORK_POLICY.md no longer documents the 'network-status' recipe",
    },
    {
        "label": "shipped opt-in overclaim",
        "edit": [
            (
                POLICY,
                "### The CLI inspect/revoke recipe",
                "### The CLI inspect/revoke recipe\n\nUser opt-in is shipped.",
            )
        ],
        "expect": "NETWORK_POLICY.md overclaims 'user opt-in is shipped'",
    },
    {
        "label": "CLI recipes unwired from dispatch",
        "edit": [
            (
                COMMAND_RS,
                "crate::network::network_status(input, out, err)",
                "crate::network::network_status(\"\", out, err)",
            ),
            (
                COMMAND_RS,
                "crate::network::network_revoke(input, out, err)",
                "crate::network::network_revoke(\"\", out, err)",
            ),
        ],
        "expect": "command.rs no longer contains 'crate::network::network_status(input, out, err)'",
    },
    {
        "label": "gate-equivalence overclaim copied into a consumer",
        "edit": [
            (
                SECURITY,
                "Accessibility semantics are read-only by default",
                "Either one is enough to stop a connection. Accessibility semantics are read-only by default",
            )
        ],
        "expect": "SECURITY.md overclaims 'either one is enough'",
    },
    {
        "label": "honest update-check exception wording removed",
        "edit": [
            (
                POLICY,
                "is stopped only by the runtime switch",
                "is stopped by either gate",
            )
        ],
        "expect": "NETWORK_POLICY.md no longer states 'stopped only by the runtime switch'",
    },
)


def _apply(tmp: Path, case: dict[str, object]) -> None:
    for relative in case.get("delete", ()):  # type: ignore[union-attr]
        (tmp / str(relative)).unlink(missing_ok=True)
    for relative, old, new in case.get("edit", ()):  # type: ignore[union-attr]
        path = tmp / str(relative)
        text = path.read_text(encoding="utf-8")
        if old == new:
            raise AssertionError(f"case is a no-op: {old!r} in {relative}")
        if old not in text:
            raise AssertionError(f"case no longer applies: {old!r} not in {relative}")
        path.write_text(text.replace(old, new, 1), encoding="utf-8", newline="\n")
    for relative, old, new in case.get("replace", ()):  # type: ignore[union-attr]
        path = tmp / str(relative)
        text = path.read_text(encoding="utf-8")
        if old == new:
            raise AssertionError(f"case is a no-op: {old!r} in {relative}")
        if old not in text:
            raise AssertionError(f"case no longer applies: {old!r} not in {relative}")
        path.write_text(text.replace(old, new), encoding="utf-8", newline="\n")


def self_test() -> int:
    pristine = {
        relative: (ROOT / relative).read_text(encoding="utf-8")
        for relative in collected_files(ROOT)
    }
    for case in CASES:
        for action in ("edit", "replace"):
            for relative, old, _ in case.get(action, ()):  # type: ignore[union-attr]
                if old not in pristine[str(relative)]:
                    print(
                        f"SELF-TEST FAILED: stale fixture in {case['label']}: "
                        f"{old!r} not in {relative}",
                        file=sys.stderr,
                    )
                    return 1
        for relative in case.get("delete", ()):  # type: ignore[union-attr]
            if str(relative) not in pristine:
                print(
                    f"SELF-TEST FAILED: stale deletion in {case['label']}: {relative}",
                    file=sys.stderr,
                )
                return 1

    tmp = Path(tempfile.mkdtemp(prefix="curbcut-network-policy-"))
    try:
        materialize(tmp, pristine)
        if validate(tmp):
            print("SELF-TEST FAILED: a pristine copy did not pass", file=sys.stderr)
            return 1
        print("self-test: the committed documents pass")

        for case in CASES:
            materialize(tmp, pristine)
            try:
                _apply(tmp, case)
            except AssertionError as error:
                print(
                    f"SELF-TEST FAILED: {case['label']}: {error}", file=sys.stderr
                )
                return 1
            errors = validate(tmp)
            expected = str(case["expect"])
            if not any(expected.lower() in error.lower() for error in errors):
                print(
                    f"SELF-TEST FAILED: {case['label']} was not detected with "
                    f"{expected!r} (got {errors[:2]})",
                    file=sys.stderr,
                )
                return 1

        print(
            f"self-test: {len(CASES)} named attacks were each detected by their "
            "own check"
        )
        return 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--self-test",
        action="store_true",
        help=f"apply {len(CASES)} named attacks and require detection",
    )
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    errors = validate(ROOT)
    for message in errors:
        print(f"FAIL: {message}", file=sys.stderr)
    if errors:
        print(f"{len(errors)} network-policy check(s) failed", file=sys.stderr)
        return 1
    print("the update-check exception is documented with its basis and real control")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
