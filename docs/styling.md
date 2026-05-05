# スタイルカスタマイズガイド

`tdsl render` が出力するHTMLは、組み込みテーマの切り替えとカスタムCSSの注入によって見た目を変更できます。
このガイドでは、利用可能なCSSクラス・プロパティのリファレンスと、実際のカスタマイズ例を説明します。

## 基本的な使い方

```bash
# 組み込みテーマを指定（default / dark / print / pastel）
tdsl render my_timeline.tdsl --theme dark --output out.html

# カスタムCSSファイルを注入（テーマCSSの後に適用される）
tdsl render my_timeline.tdsl --custom-css my_style.css --output out.html

# 組み合わせ（テーマをベースに独自スタイルを追加）
tdsl render my_timeline.tdsl --theme pastel --custom-css my_style.css --output out.html
```

`--custom-css` に指定したファイルの内容は、テーマCSSの直後に `<style>` タグとして注入されます。
そのため、テーマの任意のルールを上書きすることができます。

---

## 組み込みテーマ

| テーマ名 | 特徴 |
|---|---|
| `default` | 白背景・スチールブルーのspan・赤のevent_range。デフォルト |
| `dark` | ダークネイビー背景。目に優しい夜間表示向け |
| `print` | 白背景・モノクロ配色。印刷・PDF出力向け |
| `pastel` | クリーム系の柔らかい配色。プレゼン・共有向け |

---

## CSSクラスリファレンス

出力HTMLのSVGおよび周辺要素には、以下のCSSクラスが付与されています。
`--custom-css` で各クラスを上書きすることで、対応する要素の見た目を変更できます。

### コンテナ

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-timeline` | 年表全体を包むdivコンテナ | `background`, `border`, `border-radius`, `padding` |

### レーン

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-lane-band-even` | 偶数番レーンの背景帯（SVG rect） | `fill` |
| `.tdsl-lane-band-odd` | 奇数番レーンの背景帯（SVG rect） | `fill` |
| `.tdsl-lane-label` | レーン名テキスト | `font-size`（デフォルト: 13px）, `fill`（デフォルト: #333）, `font-weight` |

### 時間軸

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-axis-baseline` | 時間軸のベースライン | `stroke`, `stroke-width` |
| `.tdsl-axis-tick` | 目盛り縦線 | `stroke`, `stroke-width` |
| `.tdsl-axis-text` | 目盛りラベルテキスト | `font-size`（デフォルト: 11px）, `fill`（デフォルト: #666）|

### アイテム共通

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-item` | 各アイテムのグループ要素（`<g>`） | フォーカス時のスタイル用（`:focus-visible` 疑似クラスと組み合わせ） |
| `.tdsl-item-label` | アイテム上のテキストラベル | `font-size`（デフォルト: 11px）, `fill`（デフォルト: #fff）|

### Span（存続期間バー）

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-span` | 期間バー（SVG rect） | `fill`（デフォルト: #4682B4）, `fill-opacity`（デフォルト: 0.78）, `stroke`, `stroke-width` |

ホバー時は `.tdsl-span:hover { fill-opacity: 1; }` で不透明度が変化します。

### EventRange（範囲イベントバー）

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-event-range` | 範囲イベントバー（SVG rect） | `fill`（デフォルト: #DC143C）, `fill-opacity`（デフォルト: 0.75）, `stroke`, `stroke-width` |

### Event（点イベント）

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-event-dot` | イベントのドット（SVG circle） | `fill`（デフォルト: #333）, `stroke`（デフォルト: #fff）, `stroke-width` |
| `.tdsl-event-stem` | ドットから軸への縦線 | `stroke`（デフォルト: #aaa）, `stroke-width`, `stroke-dasharray`（デフォルト: `2 2`）|
| `.tdsl-event-hit` | ホバー用の透明なhit area | `fill: transparent`（変更不要）|

### ツールチップ

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-tooltip` | ホバー時に表示されるツールチップ | `background`, `border`, `border-radius`（デフォルト: 6px）, `color`, `font-size`, `box-shadow` |

---

## カスタマイズ例

### 例1: spanの色を変える

```css
/* span を緑系に変更 */
.tdsl-span {
  fill: #2e8b57;
  stroke: #1a5c38;
}

/* event_range を紫系に変更 */
.tdsl-event-range {
  fill: #7b68ee;
  stroke: #483d8b;
}
```

```bash
tdsl render my_timeline.tdsl --custom-css green_theme.css --output out.html
```

### 例2: フォントサイズを大きくする

```css
/* レーン名と軸ラベルを大きく */
.tdsl-lane-label {
  font-size: 16px;
  font-weight: 700;
}
.tdsl-axis-text {
  font-size: 13px;
}

/* アイテムラベルも大きく */
.tdsl-item-label {
  font-size: 13px;
}
```

### 例3: ダークテーマをベースに独自の色を追加

まず `--theme dark` でダークテーマを適用したうえで、`--custom-css` でアクセントカラーを変更します。

```css
/* darkテーマのspanをオレンジ系に変更 */
.tdsl-span {
  fill: #e07b39;
  stroke: #b85c1a;
}
.tdsl-event-range {
  fill: #c45c8b;
  stroke: #8b2a5a;
}
```

```bash
tdsl render my_timeline.tdsl --theme dark --custom-css accents.css --output out.html
```

### 例4: ツールチップのスタイルを変える

```css
/* ツールチップを角丸・影なしのシンプルなデザインに */
.tdsl-tooltip {
  background: #1a1a1a;
  color: #f0f0f0;
  border: none;
  border-radius: 3px;
  box-shadow: none;
  font-size: 13px;
}
```

---

## 注意事項

### SVGの `fill` / `stroke` はCSSの `color` とは別プロパティ

SVG要素の色指定にはCSSの `color` ではなく `fill`（塗り色）と `stroke`（輪郭色）を使います。
一般的なHTMLの `color: red;` のような指定は、SVG要素の `fill` には影響しません。

```css
/* 誤り: SVG要素には効かない */
.tdsl-span {
  color: red;
}

/* 正しい: SVG要素はfillで色を指定する */
.tdsl-span {
  fill: red;
}
```

### カスタムCSSの適用順序

HTML内のスタイルは以下の順序で適用されます。

1. ベースCSS（`EMBEDDED_CSS` — レイアウト・デフォルト色）
2. テーマCSS（`--theme` に対応する上書き）
3. カスタムCSS（`--custom-css` で指定したファイルの内容）

カスタムCSSは常に最後に適用されるため、任意のクラスを確実に上書きできます。
ただし詳細度（specificity）に注意してください。テーマ側のルールに `!important` が含まれている場合は、カスタムCSSでも `!important` が必要になることがあります。

### `fill-opacity` と `opacity` の違い

`.tdsl-span` や `.tdsl-event-range` は `fill-opacity` で半透明にしています。
要素全体を透明にする場合は `opacity` を使いますが、ラベルテキストも薄くなる点に注意してください。

```css
/* fill-opacity: barの塗り色のみ透明になる（ラベルは影響なし） */
.tdsl-span {
  fill-opacity: 0.5;
}

/* opacity: 要素全体（ラベルを含む）が透明になる */
.tdsl-span {
  opacity: 0.5;
}
```
