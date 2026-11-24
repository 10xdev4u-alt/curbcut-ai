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
is written, and the summary carries only waves, tasks, counts, rates, and
medians. A raw identifier appearing in the output is a failure, so ``--self-test``
asserts that too.

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


def hash_participant(identifier: str, salt: str) -> str:
    """Return a stable 16-character pseudonym for an identifier."""
    if not salt:
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
    """Aggregate both response files into a summary that carries no raw ids."""
    waves: dict[str, dict] = {}

    for row in _read_jsonl(sus_path):
        participant = row.get("participant")
        wave = row.get("wave")
        answers = row.get("answers")
        if not isinstance(participant, str) or not participant:
            raise ValueError("each SUS row needs a participant identifier")
        if not isinstance(wave, str) or not wave:
            raise ValueError("each SUS row needs a wave label")
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

    for row in _read_jsonl(tasks_path):
        participant = row.get("participant")
        wave = row.get("wave")
        task = row.get("task")
        success = row.get("success")
        elapsed = row.get("elapsed_ms")
        if not isinstance(participant, str) or not participant:
            raise ValueError("each task row needs a participant identifier")
        if not isinstance(wave, str) or not wave:
            raise ValueError("each task row needs a wave label")
        if not isinstance(task, str) or not task:
            raise ValueError("each task row needs a task id")
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

    summary: dict = {"waves": {}, "identifiers_hashed_only": True}
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
            if not name.startswith("task:"):
                continue
            times = sorted(stats["successful_ms"])
            entry["tasks"][name[len("task:") :]] = {
                "attempts": stats["attempts"],
                "successes": stats["successes"],
                "success_rate": (
                    stats["successes"] / stats["attempts"]
                    if stats["attempts"]
                    else None
                ),
                "median_successful_ms": statistics.median(times) if times else None,
            }
        summary["waves"][wave] = entry
    return summary


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
        expect(set(summary["waves"]) == {"baseline", "week4"}, "both waves reported")
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

        try:
            summarize(
                root / "missing.jsonl", tasks, "test-salt"
            )
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
