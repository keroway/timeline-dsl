#!/usr/bin/env python3
"""docs/error-catalog.md・docs/dsl-spec.md の DSL コード例が現行文法でパースできるか検証する。

## 2つの抽出モード

- `marker`（デフォルト）: error-catalog.md 向け。各コードフェンス内を `// 正しい` 行で
  区切り、**それ以降だけ**を検証する。
- `--mode simple`: dsl-spec.md 向け。dsl-spec.md は「誤り例」を含まないため、
  マーカー規約を使わずフェンス全体をそのまま検証する。

## なぜ「全ブロック」を対象にしないか（marker モード）

error-catalog はエラーの説明が目的なので、**意図的に壊れた例（`# 誤り`）を大量に含む**。
「全コードブロックが `tdsl check` を通ること」を条件にすると、誤り例まで直させることになり
カタログとして成立しなくなる。

そこで各コードフェンス内を `// 正しい` 行で区切り、**それ以降だけ**を検証する。
`// 正しい` が無いフェンス（誤り例だけを示すもの）は対象外。

## なぜ `#` ではなく `//` か

この DSL のコメントは `//` と `/* ... */` だけで、**`#` はコメントではない**
（`grammar.pest` の `COMMENT` ルール）。以前は区切りに `#` を使い、この
スクリプトが `#` 行を除去してからパースしていた。結果として:

- チェッカーは全件緑
- 一方で**利用者がフェンスをそのままコピーすると構文エラー**

という状態を作っていた（PR #791 のレビューで発覚）。**チェッカーが入力を
正規化して落としている行は、その分だけ検証されていない。** 区切り自体を
DSL の正当なコメントにすれば、除去処理そのものが不要になり前提が 1 つ減る。

## 前提

`// 誤り` / `// 正しい` というコメント行を区切りとして使う、というカタログの
書式に依存している。この規約を変えるときはこのスクリプトも合わせて変更すること。

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
# 例で使われている lane 名。カタログ・仕様書の例は lane 宣言を省略して item だけを示すことが
# 多いため、よく使われる名前をまとめて宣言しておく。
# 新しい lane 名を使う例を足したらここにも追加する（追加を忘れると
# 「Unknown lane reference」で落ちるので、黙って見逃されることはない）。
_LANES = [
    "dynasty",
    "lane",
    "a",
    "events",
    "mission",
    # dsl-spec.md の span/event/event_range/map/expand 例で使われる lane 名。
    "han",
    "ww2",
    "reiwa",
    "ongoing_conflict",
    "offices",
]

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

# `re.M` が要る。これが無いと**先頭行が文でないと一致しない**ため、
# 説明コメントで始まる例（`// 正しい（…` の続き等）が丸ごと検証対象から
# 外れる。区切りマーカー以外のコメントを落とすのをやめた際に判明した。
#
# `event_range` は先に置く：`event` を先に試すと `event\b` の語境界判定で
# `event_range` の `_` が単語構成文字のため一致せず、`event_range` 単独の
# フェンスが丸ごと検証対象から漏れる（dsl-spec.md 対応時に発覚）。
TDSL_START = re.compile(
    r"^\s*(timeline|lane|group|span|event_range|event|import|map|template|apply)\b",
    re.M,
)


def extract_correct_examples(md: Path) -> list[tuple[int, str]]:
    """(開始行, ソース) の一覧を返す。`// 正しい` 以降のみを取り出す。"""
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


def extract_simple_examples(md: Path) -> list[tuple[int, str]]:
    """(開始行, ソース) の一覧を返す。`// 正しい` マーカー無しでフェンス全体を対象にする。

    dsl-spec.md 向け。誤り例を含まないため、フェンスの中身をそのまま
    候補にし、`TDSL_START` に一致しないもの（EBNF・bash・式の断片等）だけを除く。
    """
    lines = md.read_text(encoding="utf-8").split("\n")
    out: list[tuple[int, str]] = []
    body: list[str] = []
    start = 0
    in_fence = False

    for i, line in enumerate(lines):
        if line.startswith("```"):
            if in_fence:
                src = "\n".join(body).strip()
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
    """`// 正しい` 以降の行だけを返す。

    `{ ... }` を含む例は「省略記法を示すための擬似コード」なのでスキップする。
    実際にパースできる形へ書き換えると、伝えたい 1 点がブロックの中身に埋もれる。
    """
    for idx, line in enumerate(body):
        if line.strip().startswith("// 正しい"):
            rest = body[idx + 1 :]
            # コメント行は落とさない。`//` は DSL の正当なコメントなので、
            # **フェンスの中身をそのままパーサへ渡す**。落とすと
            # 「落とした行が実は不正だった」を検出できなくなる。
            src = "\n".join(rest).strip()
            if "{ ... }" in src or "{...}" in src:
                return ""
            return src
    return ""


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default="./target/debug/tdsl")
    ap.add_argument(
        "--mode",
        choices=["marker", "simple"],
        default="marker",
        help="marker: `// 正しい` 区切り（error-catalog.md 向け）。"
        " simple: フェンス全体を検証（dsl-spec.md 向け）。",
    )
    ap.add_argument("files", nargs="*", default=["docs/error-catalog.md"])
    args = ap.parse_args()

    extract = extract_simple_examples if args.mode == "simple" else extract_correct_examples

    failures = 0
    checked = 0
    for name in args.files:
        md = Path(name)
        for start, src in extract(md):
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
