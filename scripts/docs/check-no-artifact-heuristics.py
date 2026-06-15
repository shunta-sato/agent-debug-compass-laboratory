#!/usr/bin/env python3
"""Reject public handoff docs that teach filename-order artifact selection."""

from __future__ import annotations

from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
SCAN_ROOTS = [
    ROOT / "README.md",
    ROOT / "docs" / "reference" / "cli.md",
    ROOT / "docs" / "getting-started",
    ROOT / "docs" / "testing",
    ROOT / "examples",
    ROOT / "plans",
    ROOT / "tests" / "fixtures" / "generated-agent-instructions",
    ROOT / "crates" / "adc-lab" / "tests" / "fixtures" / "generated-agent-instructions",
]
ORDERED_GLOB_RE = re.compile(
    r"(PLAN-\*\.json|APPROVAL-\*\.json|LEASE-\*\.json).*(\|\s*sort|\bsort\b|\|\s*tail\s+-n\s+1|\btail\s+-n\s+1\b)",
    re.IGNORECASE,
)
HEURISTIC_WITH_ARTIFACT_RE = re.compile(
    r"\b(find|sort|tail\s+-n\s+1|ls\s+-t)\b.*\b(artifact|plan|approval|lease|control|governor_sweep_policy|collect_plan|run_validation)\b",
    re.IGNORECASE,
)
ARTIFACT_WITH_HEURISTIC_RE = re.compile(
    r"\b(artifact|plan|approval|lease|control|governor_sweep_policy|collect_plan|run_validation)\b.*\b(find|sort|tail\s+-n\s+1|ls\s+-t)\b",
    re.IGNORECASE,
)
LATEST_ARTIFACT_RE = re.compile(
    r"\b(latest|newest)\s+(plan|approval|lease|artifact)\b",
    re.IGNORECASE,
)
MTIME_ARTIFACT_RE = re.compile(
    r"\bmtime\b.*\b(plan|approval|lease|artifact|control)\b|\b(plan|approval|lease|artifact|control)\b.*\bmtime\b",
    re.IGNORECASE,
)
NEGATIVE_CONTEXT_RE = re.compile(
    r"\b(do not|don't|must not|must never|never|not use|forbid|forbidden|avoid|reject|guard|heuristic|unsafe|stale|failure mode|without teaching|without shell-level|no public docs|no longer)\b",
    re.IGNORECASE,
)
SKIP_DIRS = {".git", "target", "lab"}
SCAN_SUFFIXES = {".md", ".yaml", ".yml", ".sh", ".txt"}


def iter_files(path: Path):
    if path.is_file():
        yield path
        return
    if not path.exists():
        return
    for child in path.rglob("*"):
        if any(part in SKIP_DIRS for part in child.relative_to(ROOT).parts):
            continue
        if child.name.startswith("._") or child.name == ".DS_Store":
            continue
        if child.is_file() and child.suffix in SCAN_SUFFIXES:
            yield child


def is_negative_context(lines: list[str], index: int) -> bool:
    start = max(0, index - 2)
    end = min(len(lines), index + 3)
    return bool(NEGATIVE_CONTEXT_RE.search("\n".join(lines[start:end])))


def matches_bad_artifact_heuristic(line: str) -> bool:
    return any(
        pattern.search(line)
        for pattern in [
            ORDERED_GLOB_RE,
            HEURISTIC_WITH_ARTIFACT_RE,
            ARTIFACT_WITH_HEURISTIC_RE,
            LATEST_ARTIFACT_RE,
            MTIME_ARTIFACT_RE,
        ]
    )


def main() -> int:
    violations: list[str] = []
    for root in SCAN_ROOTS:
        for path in iter_files(root):
            lines = path.read_text(encoding="utf-8").splitlines()
            for index, line in enumerate(lines):
                if matches_bad_artifact_heuristic(line) and not is_negative_context(
                    lines, index
                ):
                    violations.append(
                        f"{path.relative_to(ROOT)}:{index + 1}: {line.strip()}"
                    )
    if violations:
        print("docs artifact heuristic guard: fail")
        print(
            "Do not teach Agents to select plan/approval/control artifacts by filename order, mtimes, or latest/newest wording."
        )
        for violation in violations:
            print(violation)
        return 1
    print("docs artifact heuristic guard: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
