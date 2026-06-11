#!/usr/bin/env python3
"""Check Rust production file line budgets.

Phase 0 runs this target in informational mode. Later phases can pass
--enforce once the planned module splits have landed.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


DEFAULT_BUDGET = 1500
OVERRIDES = {
    "crates/adc-lab/src/main.rs": 800,
    "crates/adc-lab-core/src/report.rs": 900,
}


def rust_source_files(root: Path) -> list[Path]:
    return sorted((root / "crates").glob("*/src/**/*.rs"))


def count_lines(path: Path) -> int:
    with path.open(encoding="utf-8") as handle:
        return sum(1 for _ in handle)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--enforce", action="store_true", help="fail on budget violations")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    violations = []
    checked = 0
    for path in rust_source_files(root):
        rel = path.relative_to(root).as_posix()
        budget = OVERRIDES.get(rel, DEFAULT_BUDGET)
        lines = count_lines(path)
        checked += 1
        if lines > budget:
            violations.append((rel, lines, budget))

    mode = "enforced" if args.enforce else "informational"
    print(f"file budgets: {mode} checked={checked} violations={len(violations)}")
    for rel, lines, budget in violations:
        print(f"file budget: {rel} lines={lines} budget={budget}")

    if args.enforce and violations:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
