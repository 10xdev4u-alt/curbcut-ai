#!/usr/bin/env python3
"""Score the CurbCut longitudinal outcome study without leaking participants.

The study instrument lives in ``docs/STUDY_PROTOCOL.md``. This script turns the
two response files it defines into aggregate numbers:

* ``sus`` — ten System Usability Scale answers per participant per wave. Scored
  with the published method: odd items contribute ``answer - 1``, even items
  ``5 - answer``, the sum is multiplied by 2.5, giving 0..100.
* ``tasks`` — task attempts with a success flag and a time. Summarized as
  attempts, successes, success rate, and median successful time per wave.

Participant identifiers are reduced to a salted SHA-256 prefix before anything
is written. The summary carries wave-level aggregates plus per-participant rows
under those pseudonyms, so paired statistics can be computed without holding
identifiers. Wave and task labels come from closed sets fixed by the protocol;
a raw identifier or free-form label appearing in the output is a failure, which
``--self-test`` asserts.

Usage::

    python3 scripts/study_instrument.py summarize \
        --sus responses/sus.jsonl --tasks responses/tasks.jsonl \
        --salt "$CURBCUT_STUDY_SALT" --output build/study-summary.json

The salt is never stored. If it is lost, old summaries can no longer be joined
to new ones, which is the intended cost of not keeping identifiers.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
import sys
from pathlib import Path

SUS_ITEMS = 10

# Fixed by docs/STUDY_PROTOCOL.md §4. Free-form labels here would let an
# identifier ride into the output as a key.
WAVES = {"baseline", "week2", "week4", "week8", "week12"}
TASKS = {"book-doctor", "file-expense", "read-policy"}


def hash_participant(identifier: str, salt: str) -> str:
    """Return a stable 16-character pseudonym for an identifier."""
    if not salt or not salt.strip():
        raise ValueError("a non-empty study salt is required")
    digest = hashlib.sha256(f"{salt}\0{identifier}".encode("utf-8")).hexdigest()
    return digest[:16]


def score_sus(answers: list[int]) -> float | None:
    """Score one completed SUS form. ``None`` means the form is incomplete.

    Raises ``ValueError`` for an answer outside 1..5: a mistyped response must
    not silently become a low score.
    """
    if len(answers) != SUS_ITEMS:
        return None
    if any(not isinstance(answer, int) or isinstance(answer, bool) for answer in answers):
        raise ValueError("SUS answers must be integers")
    if any(answer < 1 or answer > 5 for answer in answers):
        raise ValueError("SUS answers must be between 1 and 5")
    total = 0
    for index, answer in enumerate(answers, start=1):
        total += answer - 1 if index % 2 == 1 else 5 - answer
    return total * 2.5


def _read_jsonl(path: Path) -> list[dict]:
    rows: list[dict] = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.strip()
        if not stripped:
            continue
        try:
            row = json.loads(stripped)
        except json.JSONDecodeError as error:
            raise ValueError(f"{path}:{number}: invalid JSON: {error}") from error
        if not isinstance(row, dict):
            raise ValueError(f"{path}:{number}: expected an object")
        rows.append(row)
    return rows


def summarize(sus_path: Path, tasks_path: Path, salt: str) -> dict:
    """Aggregate both response files into a summary that carries no raw ids.

    Wave-level aggregates are published. Per-participant rows are retained under
    salted pseudonyms so the protocol's paired baseline-to-week-12 comparison can
    be computed; no raw identifier is written.
    """
    # A whitespace-only salt would still hash, but it is guessable, so it is
    # treated as missing rather than accepted.
    if not salt or not salt.strip():
        raise ValueError("a non-empty study salt is required")

    waves: dict[str, dict] = {}
    participants: dict[str, dict] = {}

    def person(identifier: str) -> dict:
        pseudonym = hash_participant(identifier, salt)
        return participants.setdefault(pseudonym, {})

    for row in _read_jsonl(sus_path):
        participant = row.get("participant")
        wave = row.get("wave")
        answers = row.get("answers")
        if not isinstance(participant, str) or not participant:
            raise ValueError("each SUS row needs a participant identifier")
        if wave not in WAVES:
            raise ValueError(f"unknown wave label {wave!r}; expected one of {sorted(WAVES)}")
        if not isinstance(answers, list):
            raise ValueError("each SUS row needs an answers array")
        score = score_sus(answers)
        bucket = waves.setdefault(
            wave, {"sus_forms": 0, "sus_incomplete": 0, "sus_scores": []}
        )
        bucket["sus_forms"] += 1
        if score is None:
            bucket["sus_incomplete"] += 1
        else:
            bucket["sus_scores"].append(score)
        participant_bucket = person(participant).setdefault(wave, {"sus": [], "tasks": {}})
        if score is not None:
            participant_bucket["sus"].append(score)

    for row in _read_jsonl(tasks_path):
        participant = row.get("participant")
        wave = row.get("wave")
        task = row.get("task")
        success = row.get("success")
        elapsed = row.get("elapsed_ms")
        if not isinstance(participant, str) or not participant:
            raise ValueError("each task row needs a participant identifier")
        if wave not in WAVES:
            raise ValueError(f"unknown wave label {wave!r}; expected one of {sorted(WAVES)}")
        if task not in TASKS:
            raise ValueError(f"unknown task id {task!r}; expected one of {sorted(TASKS)}")
        if not isinstance(success, bool):
            raise ValueError("each task row needs a boolean success flag")
        if not isinstance(elapsed, int) or isinstance(elapsed, bool) or elapsed < 0:
            raise ValueError("each task row needs a non-negative integer elapsed_ms")
        bucket = waves.setdefault(
            wave, {"sus_forms": 0, "sus_incomplete": 0, "sus_scores": []}
        )
        key = f"task:{task}"
        task_stats = bucket.setdefault(
            key, {"attempts": 0, "successes": 0, "successful_ms": []}
        )
        task_stats["attempts"] += 1
        if success:
            task_stats["successes"] += 1
            task_stats["successful_ms"].append(elapsed)
        participant_bucket = person(participant).setdefault(wave, {"sus": [], "tasks": {}})
        personal_task = participant_bucket["tasks"].setdefault(
            task, {"attempts": 0, "successes": 0, "successful_ms": []}
        )
        personal_task["attempts"] += 1
        if success:
            personal_task["successes"] += 1
            personal_task["successful_ms"].append(elapsed)

    def task_entry(stats: dict) -> dict:
        times = sorted(stats["successful_ms"])
        return {
            "attempts": stats["attempts"],
            "successes": stats["successes"],
            "success_rate": (
                stats["successes"] / stats["attempts"] if stats["attempts"] else None
            ),
            "median_successful_ms": statistics.median(times) if times else None,
        }

    summary: dict = {
        "identifiers_hashed_only": True,
        "waves": {},
        "participants": {},
        "paired": {},
    }
    for wave, bucket in sorted(waves.items()):
        entry = {
            "sus_forms": bucket["sus_forms"],
            "sus_incomplete": bucket["sus_incomplete"],
            "sus_median": (
                statistics.median(bucket["sus_scores"])
                if bucket["sus_scores"]
                else None
            ),
            "tasks": {},
        }
        for name, stats in sorted(bucket.items()):
            if name.startswith("task:"):
                entry["tasks"][name[len("task:") :]] = task_entry(stats)
        summary["waves"][wave] = entry

    for pseudonym, by_wave in sorted(participants.items()):
        serialized_waves = {}
        for wave, bucket in sorted(by_wave.items()):
            serialized_waves[wave] = {
                "sus": sorted(bucket["sus"]),
                "tasks": {
                    task: task_entry(stats)
                    for task, stats in sorted(bucket["tasks"].items())
                },
            }
        summary["participants"][pseudonym] = {"waves": serialized_waves}

    summary["paired"]["baseline_to_week12"] = paired_change(participants)
    return summary


def paired_change(participants: dict[str, dict]) -> dict:
    """Paired SUS change from baseline to week 12.

    Only participants with exactly one completed form at each wave are paired.
    The interval is a normal approximation (mean +/- 1.96 standard errors); with
    a sample this small it is indicative, not exact, and ``None`` below 2 pairs.
    """
    differences: list[float] = []
    for by_wave in participants.values():
        baseline = by_wave.get("baseline", {}).get("sus", [])
        end = by_wave.get("week12", {}).get("sus", [])
        if len(baseline) == 1 and len(end) == 1:
            differences.append(end[0] - baseline[0])
    result: dict = {"count": len(differences)}
    if not differences:
        result.update({"mean": None, "sd": None, "ci95": None})
        return result
    mean = statistics.fmean(differences)
    sd = statistics.stdev(differences) if len(differences) > 1 else None
    result["mean"] = mean
    result["sd"] = sd
    if sd is None:
        result["ci95"] = None
    else:
        half = 1.96 * sd / (len(differences) ** 0.5)
        result["ci95"] = [mean - half, mean + half]
    return result


def _self_test() -> int:
    checks = 0

    def expect(condition: bool, label: str) -> None:
        nonlocal checks
        if not condition:
            print(f"SELF-TEST FAILED: {label}", file=sys.stderr)
            raise SystemExit(1)
        checks += 1

    # Every item at the midpoint contributes 2, so a flat form is 50.
    expect(score_sus([1] * 10) == 50.0, "all-ones SUS scores 50")
    expect(score_sus([3] * 10) == 50.0, "midpoint SUS scores 50")
    expect(score_sus([5] * 10) == 50.0, "all-fives SUS scores 50")
    # Odd 1, even 5 contributes (1-1) + (5-5) = 0; odd 5, even 1 = 4+4 = 8.
    alternating = [1 if index % 2 == 1 else 5 for index in range(1, 11)]
    expect(score_sus(alternating) == 0.0, "alternating low/high SUS is 0")
    reversed_alt = [5 if index % 2 == 1 else 1 for index in range(1, 11)]
    expect(score_sus(reversed_alt) == 100.0, "alternating high/low SUS is 100")
    expect(score_sus([3] * 9) is None, "incomplete SUS is None")
    for bad in ([0] + [3] * 9, [6] + [3] * 9, ["3"] * 10, [True] * 10):
        try:
            score_sus(bad)
        except ValueError:
            checks += 1
        else:
            print(f"SELF-TEST FAILED: {bad!r} was accepted", file=sys.stderr)
            raise SystemExit(1)

    first = hash_participant("participant-a", "salt-one")
    second = hash_participant("participant-a", "salt-one")
    other = hash_participant("participant-b", "salt-one")
    changed = hash_participant("participant-a", "salt-two")
    expect(first == second, "hashing is stable for the same salt")
    expect(first != other, "different identifiers hash differently")
    expect(first != changed, "changing the salt changes the pseudonym")
    expect(len(first) == 16, "pseudonyms are 16 characters")
    try:
        hash_participant("participant-a", "")
    except ValueError:
        checks += 1
    else:
        print("SELF-TEST FAILED: an empty salt was accepted", file=sys.stderr)
        raise SystemExit(1)

    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        sus = root / "sus.jsonl"
        tasks = root / "tasks.jsonl"
        sus.write_text(
            "\n".join(
                [
                    json.dumps(
                        {"participant": "participant-a", "wave": "baseline",
                         "answers": [5, 1, 5, 1, 5, 1, 5, 1, 5, 1]}
                    ),
                    json.dumps(
                        {"participant": "participant-b", "wave": "baseline",
                         "answers": [1, 5, 1, 5, 1, 5, 1, 5, 1, 5]}
                    ),
                    json.dumps(
                        {"participant": "participant-c", "wave": "week4",
                         "answers": [3, 3, 3]}
                    ),
                    json.dumps(
                        {"participant": "participant-a", "wave": "week12",
                         "answers": [3, 3, 3, 3, 3, 3, 3, 3, 3, 3]}
                    ),
                    json.dumps(
                        {"participant": "participant-b", "wave": "week12",
                         "answers": [3, 3, 3, 3, 3, 3, 3, 3, 3, 3]}
                    ),
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        tasks.write_text(
            "\n".join(
                [
                    json.dumps(
                        {"participant": "participant-a", "wave": "baseline",
                         "task": "book-doctor", "success": True, "elapsed_ms": 1200}
                    ),
                    json.dumps(
                        {"participant": "participant-a", "wave": "baseline",
                         "task": "book-doctor", "success": True, "elapsed_ms": 800}
                    ),
                    json.dumps(
                        {"participant": "participant-b", "wave": "baseline",
                         "task": "book-doctor", "success": False, "elapsed_ms": 3000}
                    ),
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        summary = summarize(sus, tasks, "test-salt")
        rendered = json.dumps(summary)
        expect("participant-a" not in rendered, "raw identifiers stay out of output")
        expect(
            set(summary["waves"]) == {"baseline", "week4", "week12"},
            "all three waves reported",
        )
        baseline = summary["waves"]["baseline"]
        expect(baseline["sus_forms"] == 2, "both complete forms counted")
        expect(baseline["sus_median"] == 50.0, "median SUS of 100 and 0 is 50")
        task = baseline["tasks"]["book-doctor"]
        expect(task["attempts"] == 3, "all attempts counted")
        expect(task["successes"] == 2, "successes counted")
        expect(abs(task["success_rate"] - 2 / 3) < 1e-9, "rate is successes/attempts")
        expect(task["median_successful_ms"] == 1000.0, "median successful time")
        week4 = summary["waves"]["week4"]
        expect(week4["sus_incomplete"] == 1, "incomplete form counted separately")
        expect(week4["sus_median"] is None, "no median without complete forms")

        pseudonym = hash_participant("participant-a", "test-salt")
        expect(pseudonym in summary["participants"], "pseudonym row exists")
        personal = summary["participants"][pseudonym]["waves"]
        expect(personal["baseline"]["sus"] == [100.0], "paired SUS kept per participant")
        expect(summary["identifiers_hashed_only"] is True, "hashed-only flag set")
        paired = summary["paired"]["baseline_to_week12"]
        expect(paired["count"] == 2, "both participants pair baseline to week12")
        expect(abs(paired["mean"]) < 1e-9, "mean change of -50 and +50 is zero")
        expect(paired["sd"] is not None and paired["ci95"] is not None,
               "interval reported for two pairs")
        expect(abs(paired["ci95"][0] + paired["ci95"][1]) < 1e-9,
               "interval is centred on the mean")

        bad_wave = root / "bad-wave.jsonl"
        bad_wave.write_text(
            json.dumps(
                {"participant": "x", "wave": "week99", "answers": [3] * 10}
            )
            + "\n",
            encoding="utf-8",
        )
        for path, label in ((bad_wave, "unknown wave"),):
            try:
                summarize(path, tasks, "test-salt")
            except ValueError as error:
                expect(label in str(error), "error names the bad wave")
            else:
                print(f"SELF-TEST FAILED: {label} was accepted", file=sys.stderr)
                raise SystemExit(1)

        bad_task = root / "bad-task.jsonl"
        bad_task.write_text(
            json.dumps(
                {"participant": "x", "wave": "baseline", "task": "read-email",
                 "success": True, "elapsed_ms": 10}
            )
            + "\n",
            encoding="utf-8",
        )
        empty = root / "empty.jsonl"
        empty.write_text("", encoding="utf-8")
        try:
            summarize(empty, bad_task, "test-salt")
        except ValueError as error:
            expect("unknown task id" in str(error), "error names the bad task")
        else:
            print("SELF-TEST FAILED: an unknown task was accepted", file=sys.stderr)
            raise SystemExit(1)

        for blank in ("", "   ", "\t"):
            try:
                summarize(sus, tasks, blank)
            except ValueError as error:
                expect("salt" in str(error), f"{blank!r} salt rejected on summarize")
            else:
                print(
                    f"SELF-TEST FAILED: {blank!r} salt reached summarize",
                    file=sys.stderr,
                )
                raise SystemExit(1)

        try:
            summarize(root / "missing.jsonl", tasks, "test-salt")
        except FileNotFoundError:
            checks += 1
        else:
            print("SELF-TEST FAILED: a missing file was accepted", file=sys.stderr)
            raise SystemExit(1)

    print(f"self-test: {checks} study-instrument checks passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true", help="run the built-in checks")
    sub = parser.add_subparsers(dest="command")
    summarize_parser = sub.add_parser("summarize", help="aggregate response files")
    summarize_parser.add_argument("--sus", type=Path, required=True)
    summarize_parser.add_argument("--tasks", type=Path, required=True)
    summarize_parser.add_argument("--salt", required=True)
    summarize_parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    if args.self_test:
        return _self_test()
    if args.command != "summarize":
        parser.print_help()
        return 2

    summary = summarize(args.sus, args.tasks, args.salt)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
