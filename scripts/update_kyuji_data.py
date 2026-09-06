#!/usr/bin/env python3
"""
公開 spreadsheet の CSV から、正字變換用の snapshot JSON を生成する。

使ひ方:
    python3 .claude/skills/seiji-seikana-converter/scripts/update_kyuji_data.py
    python3 .claude/skills/seiji-seikana-converter/scripts/update_kyuji_data.py \
        --csv /path/to/source.csv
"""

from __future__ import annotations

import argparse
import csv
import json
import urllib.request
from collections import defaultdict
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

DEFAULT_SOURCE_URL = (
    "https://docs.google.com/spreadsheets/d/"
    "1CEBTf13rCCnA99Fvyg6PbBQTBRJN1JhevifxrfVGHE0/export?format=csv&gid=0"
)


@dataclass(frozen=True)
class Candidate:
    source: str
    target: str | None
    row_id: str
    candidate_no: str
    note: str


def parse_args() -> argparse.Namespace:
    script_dir = Path(__file__).resolve().parent
    default_output = script_dir.parent / "data" / "kyuji_map.json"
    parser = argparse.ArgumentParser()
    parser.add_argument("--csv", type=Path, help="既に取得した CSV を使ふ")
    parser.add_argument("--source-url", default=DEFAULT_SOURCE_URL)
    parser.add_argument("--output", type=Path, default=default_output)
    return parser.parse_args()


def fetch_csv(url: str) -> str:
    request = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read().decode("utf-8-sig")


def resolve_target(row: dict[str, str]) -> str | None:
    source = row["元字"].strip()
    code = row["正碼"].strip()
    ivs = row["正ivs"].strip()
    compat = row["換字"].strip()

    if not source or not code:
        return None
    if code == ".":
        if not ivs:
            return None
        if compat:
            return compat
        return source
    return chr(int(code, 16))


def load_rows(csv_text: str) -> list[dict[str, str]]:
    return list(csv.DictReader(csv_text.splitlines()))


def build_snapshot(
    rows: list[dict[str, str]],
    source_url: str,
    input_csv: str | None = None,
) -> dict[str, object]:
    grouped: dict[str, list[Candidate]] = defaultdict(list)
    for row in rows:
        source = row["元字"].strip()
        if not source:
            continue
        grouped[source].append(
            Candidate(
                source=source,
                target=resolve_target(row),
                row_id=row["行番號"].strip(),
                candidate_no=row["候補"].strip(),
                note=row["備考"].strip(),
            )
        )

    char_map: dict[str, str] = {}
    ambiguous: dict[str, list[dict[str, str | None]]] = {}
    unchanged: list[str] = []

    for source, candidates in grouped.items():
        targets = {candidate.target for candidate in candidates}
        if len(targets) == 1:
            only = next(iter(targets))
            if only is None:
                unchanged.append(source)
            else:
                char_map[source] = only
            continue

        ambiguous[source] = [
            {
                "candidate_no": candidate.candidate_no or "",
                "target": candidate.target,
                "row_id": candidate.row_id,
                "note": candidate.note or "",
            }
            for candidate in candidates
        ]

    return {
        "source_url": source_url,
        "generated_at": datetime.now(UTC).isoformat(),
        "row_count": len(rows),
        "input_csv": input_csv,
        "char_map": dict(sorted(char_map.items())),
        "ambiguous_characters": dict(sorted(ambiguous.items())),
        "unchanged_characters": sorted(unchanged),
    }


def main() -> int:
    args = parse_args()
    if args.csv:
        csv_text = args.csv.read_text(encoding="utf-8-sig")
        input_csv = str(args.csv)
    else:
        csv_text = fetch_csv(args.source_url)
        input_csv = None

    snapshot = build_snapshot(
        load_rows(csv_text),
        args.source_url,
        input_csv=input_csv,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(snapshot, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {args.output}")
    print(
        "char_map={char_map} ambiguous={ambiguous} unchanged={unchanged}".format(
            char_map=len(snapshot["char_map"]),
            ambiguous=len(snapshot["ambiguous_characters"]),
            unchanged=len(snapshot["unchanged_characters"]),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
