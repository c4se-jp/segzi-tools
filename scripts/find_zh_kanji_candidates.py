#!/usr/bin/env python3
"""
简化字・繁體字 -> 日本語漢字 の變換表を育てるための候補抽出。

Unicode 公式の Unihan データベース (kSimplifiedVariant / kTraditionalVariant) を
既存の kyuji_map.json (日本語の新字体<->正字對應) と突き合はせ、
まだ zh_char_map.tsv / zh_compound_map.tsv / zh_ambiguous_characters.json に
載ってゐない文字を、實際に指定したコーパスへ出現する頻度順で報吿する。

候補はあくまで報吿のみ。誤爆(日本語として正規に使はれる字を誤って
變換してしまふこと)を避けるため、必ず人手で個別に検討してから
zh_char_map.tsv 等へ追記すること。

使ひ方:
    python3 scripts/find_zh_kanji_candidates.py \
        --scan-dir wiki

既に取得濟みの Unihan_Variants.txt を使ふ場合:
    python3 scripts/find_zh_kanji_candidates.py \
        --scan-dir wiki --unihan-variants /path/to/Unihan_Variants.txt
"""

from __future__ import annotations

import argparse
import io
import json
import urllib.request
import zipfile
from collections import Counter
from pathlib import Path

UNIHAN_ZIP_URL = "https://www.unicode.org/Public/UCD/latest/ucd/Unihan.zip"


def parse_args() -> argparse.Namespace:
    script_dir = Path(__file__).resolve().parent
    data_dir = script_dir.parent / "dic"
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--scan-dir",
        type=Path,
        action="append",
        required=True,
        help="候補の出現頻度を數へる對象ディレクトリ(複數指定可)",
    )
    parser.add_argument("--data-dir", type=Path, default=data_dir)
    parser.add_argument(
        "--unihan-variants",
        type=Path,
        help="取得濟みの Unihan_Variants.txt を使ふ。未指定なら unicode.org から取得する",
    )
    parser.add_argument(
        "--min-count",
        type=int,
        default=1,
        help="コーパス中の最小出現囘數。既定は1",
    )
    parser.add_argument("--output", type=Path, help="TSV の出力先。未指定なら stdout")
    return parser.parse_args()


def fetch_unihan_variants() -> str:
    request = urllib.request.Request(
        UNIHAN_ZIP_URL, headers={"User-Agent": "Mozilla/5.0"}
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        blob = response.read()
    with zipfile.ZipFile(io.BytesIO(blob)) as archive:
        with archive.open("Unihan_Variants.txt") as fh:
            return fh.read().decode("utf-8")


def to_char(codepoint: str) -> str:
    return chr(int(codepoint[2:], 16))


def load_variant_pairs(variants_text: str) -> dict[str, set[str]]:
    """简化字 -> 對應する繁體字(候補)の集合。

    kTraditionalVariant は「この字は简化字で、繁體字はこちら」を示す。
    kSimplifiedVariant はその逆向き(繁體字 -> 简化字)なので、
    ここでは候補抽出の方向を誤らないやう kTraditionalVariant のみ使ふ。
    """
    pairs: dict[str, set[str]] = {}
    for line in variants_text.splitlines():
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) < 3:
            continue
        src_code, field, values = parts[0], parts[1], parts[2]
        if field != "kTraditionalVariant":
            continue
        src = to_char(src_code)
        for token in values.split():
            code = token.split("<")[0]
            if not code.startswith("U+"):
                continue
            pairs.setdefault(src, set()).add(to_char(code))
    return pairs


def load_known_sources(data_dir: Path) -> set[str]:
    known: set[str] = set()

    kyuji = json.loads((data_dir / "kyuji_map.json").read_text(encoding="utf-8"))
    known.update(kyuji["char_map"].keys())
    known.update(kyuji["char_map"].values())
    known.update(kyuji["ambiguous_characters"].keys())

    for name in ("zh_char_map.tsv", "zh_compound_map.tsv"):
        path = data_dir / name
        if not path.exists():
            continue
        for raw_line in path.read_text(encoding="utf-8").splitlines():
            line = raw_line.strip()
            if not line or line.startswith("#"):
                continue
            known.add(line.split("\t")[0])

    ambiguous_path = data_dir / "zh_ambiguous_characters.json"
    if ambiguous_path.exists():
        known.update(json.loads(ambiguous_path.read_text(encoding="utf-8")).keys())

    return known


def resolve_ja_candidates(
    char: str, variant_pairs: dict[str, set[str]], reverse_kyuji: dict[str, str]
) -> set[str]:
    candidates: set[str] = set()
    for partner in variant_pairs.get(char, set()):
        if partner == char:
            continue
        candidates.add(reverse_kyuji.get(partner, partner))
    return candidates


def count_corpus_chars(scan_dirs: list[Path], targets: set[str]) -> Counter[str]:
    counter: Counter[str] = Counter()
    for scan_dir in scan_dirs:
        for path in scan_dir.rglob("*.md"):
            text = path.read_text(encoding="utf-8", errors="ignore")
            for ch in text:
                if ch in targets:
                    counter[ch] += 1
    return counter


def main() -> int:
    args = parse_args()

    variants_text = (
        args.unihan_variants.read_text(encoding="utf-8")
        if args.unihan_variants
        else fetch_unihan_variants()
    )
    variant_pairs = load_variant_pairs(variants_text)

    kyuji = json.loads((args.data_dir / "kyuji_map.json").read_text(encoding="utf-8"))
    reverse_kyuji: dict[str, str] = {}
    for shinjitai, seiji in kyuji["char_map"].items():
        reverse_kyuji.setdefault(seiji, shinjitai)

    known_sources = load_known_sources(args.data_dir)

    unknown_variant_chars = {c for c in variant_pairs if c not in known_sources}
    counter = count_corpus_chars(args.scan_dir, unknown_variant_chars)

    rows = []
    for char, count in counter.most_common():
        if count < args.min_count:
            continue
        candidates = resolve_ja_candidates(char, variant_pairs, reverse_kyuji)
        candidates.discard(char)
        if not candidates:
            continue
        rows.append((char, count, sorted(candidates)))

    lines = ["# char\tcount\tja_candidates(要人手選別)"]
    for char, count, candidates in rows:
        lines.append(f"{char}\t{count}\t{'|'.join(candidates)}")
    output_text = "\n".join(lines) + "\n"

    if args.output:
        args.output.write_text(output_text, encoding="utf-8")
        print(f"wrote {args.output} ({len(rows)} candidates)")
    else:
        print(output_text, end="")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
