#!/usr/bin/env python3
"""Reject docs/examples that teach filename-order artifact selection."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCAN_ROOTS = [ROOT / "README.md", ROOT / "docs", ROOT / "examples"]
HEURISTIC_RE = re.compile(r"\b(find|sort|tail|ls\s+-t)\b", re.IGNORECASE)
ARTIFACT_RE = re.compile(
    r"(PLAN-|APPROVAL-|approval|approvals|plan|plans|control|\.result\.json|governor_sweep_policy)",
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
        if child.is_file() and child.suffix in SCAN_SUFFIXES:
            yield child


def main() -> int:
    violations: list[str] = []
    for root in SCAN_ROOTS:
        for path in iter_files(root):
            text = path.read_text(encoding="utf-8")
            for index, line in enumerate(text.splitlines(), start=1):
                if HEURISTIC_RE.search(line) and ARTIFACT_RE.search(line):
                    violations.append(f"{path.relative_to(ROOT)}:{index}: {line.strip()}")
    if violations:
        print("docs artifact heuristic guard: fail")
        print(
            "Do not teach Agents to select plan/approval/control artifacts by find/sort/tail/ls -t."
        )
        for violation in violations:
            print(violation)
        return 1
    print("docs artifact heuristic guard: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
