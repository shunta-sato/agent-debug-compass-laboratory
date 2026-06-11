#!/usr/bin/env python3
"""Validate schema-ledger.tsv coverage.

This is intentionally dependency-free. It checks that every top-level schema
file is classified and that schema-versioned v1 contracts without schema files
are not invisible to the ledger.
"""

from __future__ import annotations

import argparse
import csv
import re
import sys
from pathlib import Path


ALLOWED_CURRENT_STATES = {
    "handwritten_schema",
    "generated_snapshot",
    "no_schema_wire_contract",
    "deleted",
}
ALLOWED_TARGET_STATES = {"delete", "generated_checked", "exempt"}
WIRE_RE = re.compile(r"lab\.[a-z0-9_]+(?:\.[a-z0-9_]+)*\.v1(?:\.[a-z0-9_]+)*")
IGNORED_SUFFIXES = (".schema.json", ".valid.json")
SCAN_DIRS = ("crates", "tests", "examples")


def load_ledger(path: Path) -> dict[str, dict[str, str]]:
    rows: dict[str, dict[str, str]] = {}
    with path.open(newline="", encoding="utf-8") as handle:
        filtered = (line for line in handle if line.strip() and not line.startswith("#"))
        reader = csv.DictReader(filtered, delimiter="\t")
        required = {
            "contract_id",
            "schema_file",
            "current_state",
            "target_state",
            "phase",
            "owner",
            "notes",
        }
        if set(reader.fieldnames or []) != required:
            raise ValueError(f"{path} header must be: {sorted(required)}")
        for row in reader:
            contract_id = row["contract_id"]
            if contract_id in rows:
                raise ValueError(f"duplicate contract_id in ledger: {contract_id}")
            rows[contract_id] = row
    return rows


def top_level_schema_contracts(schema_dir: Path) -> dict[str, str]:
    contracts = {}
    for path in sorted(schema_dir.glob("*.schema.json")):
        contract_id = path.name.removesuffix(".schema.json")
        contracts[contract_id] = path.name
    return contracts


def schema_file_exists(root: Path, schema_file: str) -> bool:
    return (root / "schemas" / schema_file).exists()


def discovered_v1_contracts(root: Path) -> set[str]:
    contracts: set[str] = set()
    for dirname in SCAN_DIRS:
        scan_root = root / dirname
        if not scan_root.exists():
            continue
        for path in scan_root.rglob("*"):
            if not path.is_file() or path.suffix not in {".rs", ".json", ".yaml", ".yml", ".md"}:
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue
            for match in WIRE_RE.findall(text):
                if match.endswith(IGNORED_SUFFIXES):
                    continue
                contracts.add(match)
    return contracts


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument(
        "--ledger", default="schemas/schema-ledger.tsv", help="schema ledger path"
    )
    parser.add_argument(
        "--enforce-final",
        action="store_true",
        help="fail if any maintained-by-hand schema remains",
    )
    args = parser.parse_args()

    root = Path(args.root).resolve()
    ledger = load_ledger(root / args.ledger)
    schema_contracts = top_level_schema_contracts(root / "schemas")
    discovered = discovered_v1_contracts(root)
    no_schema_contracts = discovered - set(schema_contracts)

    errors: list[str] = []
    for contract_id, schema_file in schema_contracts.items():
        row = ledger.get(contract_id)
        if row is None:
            errors.append(f"missing ledger row for top-level schema: {schema_file}")
            continue
        if row["schema_file"] != schema_file:
            errors.append(
                f"{contract_id}: schema_file must be {schema_file}, got {row['schema_file']}"
            )
        if row["current_state"] not in {"handwritten_schema", "generated_snapshot"}:
            errors.append(
                f"{contract_id}: top-level schema current_state cannot be {row['current_state']}"
            )

    for contract_id in sorted(no_schema_contracts):
        row = ledger.get(contract_id)
        if row is None:
            errors.append(f"missing ledger row for no-schema wire contract: {contract_id}")
            continue
        if row["current_state"] == "no_schema_wire_contract":
            if row["schema_file"] != "-":
                errors.append(f"{contract_id}: no-schema wire contract must use schema_file '-'")
        elif row["current_state"] == "generated_snapshot":
            if row["schema_file"] == "-":
                errors.append(f"{contract_id}: generated snapshot row must name a schema file")
            elif not schema_file_exists(root, row["schema_file"]):
                errors.append(
                    f"{contract_id}: generated snapshot file does not exist: {row['schema_file']}"
                )
        else:
            errors.append(
                f"{contract_id}: expected current_state no_schema_wire_contract or generated_snapshot, got {row['current_state']}"
            )

    for contract_id, row in ledger.items():
        if row["current_state"] not in ALLOWED_CURRENT_STATES:
            errors.append(f"{contract_id}: invalid current_state {row['current_state']}")
        if row["target_state"] not in ALLOWED_TARGET_STATES:
            errors.append(f"{contract_id}: invalid target_state {row['target_state']}")
        if row["current_state"] == "deleted":
            if row["schema_file"] == "-":
                errors.append(f"{contract_id}: deleted schema row must name the retired schema file")
            elif (root / "schemas" / row["schema_file"]).exists():
                errors.append(f"{contract_id}: deleted schema file still exists: {row['schema_file']}")
            continue
        if row["current_state"] != "no_schema_wire_contract":
            if row["schema_file"] == "-":
                errors.append(f"{contract_id}: schema-backed row must name a schema file")
            elif not schema_file_exists(root, row["schema_file"]):
                errors.append(f"{contract_id}: schema_file does not exist: {row['schema_file']}")
        elif row["schema_file"] != "-":
            errors.append(f"{contract_id}: no-schema row must use schema_file '-'")

    maintained_by_hand = [
        contract_id
        for contract_id, row in ledger.items()
        if row["current_state"] == "handwritten_schema" and row["target_state"] != "exempt"
    ]
    if args.enforce_final and maintained_by_hand:
        errors.append(
            "maintained-by-hand schemas remain: " + ", ".join(sorted(maintained_by_hand))
        )
    if args.enforce_final:
        unfinished_generated = [
            contract_id
            for contract_id, row in ledger.items()
            if row["target_state"] == "generated_checked"
            and row["current_state"] != "generated_snapshot"
        ]
        if unfinished_generated:
            errors.append(
                "generated-check targets are not generated snapshots: "
                + ", ".join(sorted(unfinished_generated))
            )

    if errors:
        for error in errors:
            print(f"schema-ledger error: {error}", file=sys.stderr)
        return 1

    print(
        "schema ledger: ok "
        f"top_level={len(schema_contracts)} "
        f"no_schema_wire={len(no_schema_contracts)} "
        f"maintained_by_hand={len(maintained_by_hand)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
