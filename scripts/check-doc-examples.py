#!/usr/bin/env python3
"""docs/error-catalog.md の「正しい」コード例が現行文法でパースできるか検証する。

## なぜ「全ブロック」を対象にしないか

error-catalog はエラーの説明が目的なので、**意図的に壊れた例（`# 誤り`）を大量に含む**。
「全コードブロックが `tdsl check` を通ること」を条件にすると、誤り例まで直させることになり
カタログとして成立しなくなる。

そこで各コードフェンス内を `# 正しい` 行で区切り、**それ以降だけ**を検証する。
`# 正しい` が無いフェンス（誤り例だけを示すもの）は対象外。

## 前提

`# 誤り` / `# 正しい` というコメント行を区切りとして使う、というカタログの既存の書式に
依存している。この規約を変えるときはこのスクリプトも合わせて変更すること。

使い方: python3 scripts/check-doc-examples.py [--bin <tdsl>] [<markdown>...]
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# lane 参照などを解決するために、例の前に付ける最小の前置き。
# 例が単体で完結していなくても「文法として妥当か」を見たいので、
# timeline / lane の宣言だけを補う。
# 例で使われている lane 名。カタログの例は lane 宣言を省略して item だけを示すことが
# 多いため、よく使われる名前をまとめて宣言しておく。
# 新しい lane 名を使う例を足したらここにも追加する（追加を忘れると
# 「Unknown lane reference」で落ちるので、黙って見逃されることはない）。
_LANES = ["dynasty", "lane", "a", "events", "mission"]

_LANE_DECLS = "\n\n".join(
    f'lane "{name}" as {name} {{\n  kind custom;\n  order {i + 1};\n}}'
    for i, name in enumerate(_LANES)
)

_TIMELINE = """timeline "doc-example" {
  title "doc example";
  unit year;
  range -1000..2100;
}"""

PREAMBLE = _TIMELINE + "\n\n" + _LANE_DECLS + "\n"

TDSL_START = re.compile(r"^\s*(timeline|lane|span|event|import|map)\b")


def extract_correct_examples(md: Path) -> list[tuple[int, str]]:
    """(開始行, ソース) の一覧を返す。`# 正しい` 以降のみを取り出す。"""
    lines = md.read_text(encoding="utf-8").split("\n")
    out: list[tuple[int, str]] = []
    body: list[str] = []
    start = 0
    in_fence = False

    for i, line in enumerate(lines):
        if line.startswith("```"):
            if in_fence:
                src = _correct_part(body)
                if src and TDSL_START.search(src):
                    out.append((start, src))
                body, in_fence = [], False
            else:
                in_fence, start = True, i + 2
            continue
        if in_fence:
            body.append(line)
    return out


def _correct_part(body: list[str]) -> str:
    """`# 正しい` 以降の行だけを、コメントを除いて返す。

    `{ ... }` を含む例は「省略記法を示すための擬似コード」なのでスキップする。
    実際にパースできる形へ書き換えると、伝えたい 1 点がブロックの中身に埋もれる。
    """
    for idx, line in enumerate(body):
        if line.strip().startswith("# 正しい"):
            rest = body[idx + 1 :]
            src = "\n".join(l for l in rest if not l.strip().startswith("#")).strip()
            if "{ ... }" in src or "{...}" in src:
                return ""
            return src
    return ""


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default="./target/debug/tdsl")
    ap.add_argument("files", nargs="*", default=["docs/error-catalog.md"])
    args = ap.parse_args()

    failures = 0
    checked = 0
    for name in args.files:
        md = Path(name)
        for start, src in extract_correct_examples(md):
            checked += 1
            # 前置きは「例に足りないものだけ」を補う。重複して宣言すると
            # 「Multiple timeline blocks found」「Duplicate lane alias」になる。
            has_timeline = re.search(r"^\s*timeline\b", src, re.M) is not None
            has_lane = re.search(r"^\s*lane\b", src, re.M) is not None
            parts = []
            if not has_timeline:
                parts.append(_TIMELINE)
            if not has_lane:
                parts.append(_LANE_DECLS)
            preamble = "\n\n".join(parts)
            with tempfile.NamedTemporaryFile(
                "w", suffix=".tdsl", delete=False, encoding="utf-8"
            ) as f:
                f.write(preamble + "\n" + src + "\n")
                path = f.name
            proc = subprocess.run(
                [args.bin, "check", path], capture_output=True, text=True
            )
            Path(path).unlink(missing_ok=True)
            if proc.returncode != 0:
                failures += 1
                print(f"NG {md}:{start} の「正しい」例がパースできません", file=sys.stderr)
                print(src, file=sys.stderr)
                print((proc.stderr or proc.stdout).strip()[:600], file=sys.stderr)
                print("", file=sys.stderr)

    if checked == 0:
        # 抽出が 0 件なら、規約が変わったかスクリプトが壊れている。
        # 「検査対象なし = 成功」で通すと silent-pass になる。
        print("error: 「正しい」例を 1 件も抽出できませんでした", file=sys.stderr)
        return 1

    if failures:
        print(f"{failures}/{checked} 件の「正しい」例がパースできません", file=sys.stderr)
        return 1

    print(f"OK: {checked} 件の「正しい」例がすべてパースできました")
    return 0


if __name__ == "__main__":
    sys.exit(main())
