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

出力HTMLはデフォルトで外部フォント/CDNに依存しません。本文のフォントは OS のシステムフォントスタック（例: Hiragino Sans / Yu Gothic / Meiryo / Segoe UI / Roboto）を使用します。Web フォントを使いたい場合は、必要な `@font-face` や `<link>` 相当のCSSを `--custom-css` で明示的に追加してください。

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

アイテムの塗り色は `item.color`（DSL の `color "...";`） > `color_map` の tag 色 > lane パレット色の順で決定します。`color` は hex 色（`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`）または単純な CSS 色キーワードのみを受け付けます。

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-item` | 各アイテムのグループ要素（`<g>`） | フォーカス時のスタイル用（`:focus-visible` 疑似クラスと組み合わせ） |
| `.tdsl-item-label` | アイテム上のテキストラベル | `font-size`（デフォルト: 11px）, `fill`（デフォルト: #fff）|
| `.tdsl-item-label-external` / `.tdsl-event-label` に付与 | バー外側に配置されたラベル（下記「ラベルのはみ出し対策」参照） | `font-size` はインラインの `style` 属性で上書きされる場合がある |
| `.tdsl-label-leader` | 外出しラベルとバー/ドットを結ぶリーダー線（SVG line） | `stroke`（デフォルト: #999）, `stroke-width`, `stroke-dasharray`（デフォルト: `2 2`）|

#### ラベルのはみ出し対策（#535）

`show_event_labels` が有効な場合、Span/EventRange/Event のラベルはバー幅（または利用可能な余白）に対して以下の順で調整されます。文字幅は近似計測（等幅近似、CJK/半角混在対応）に基づく判定です。

1. **フォント縮小**: 100% → 85% → 70% の順に縮小を試し、収まるサイズがあればそのまま表示する（インライン `style="font-size:…px"` を付与）。短いラベルで縮小が不要な場合は見た目は変化しない（回帰なし）。
2. **省略記号での切り詰め**: 最小サイズ（70%、下限 7px）でも収まらない場合、末尾を省略記号（`…`）で切り詰める。
3. **外出し配置**: 切り詰めても数文字も入らないほどバーが狭い場合、ラベルをバー外側に配置し、細い破線（`.tdsl-label-leader`）でバー/ドットと接続する。

縮小・切り詰め・外出しのいずれの場合も、ラベルの完全なテキストは既存の `<title>`（ツールチップ）で確認できます。

#### 隣接するEventラベルの袘噂回避（#537）

`show_event_labels` が有効な場合、同一レーン内で時間的に近接する複数の Event（点イベント）の常時表示ラベルが重ならないよう、以下の方針で自動調整されます（推定文字幅は #535 と同じ近似計測を使用）。

1. レーンごとに、描画予定のラベル矩形（時間軸上の推定幅）を時間軸順に走査し、重なりを検出する。
2. 重なりが検出されたラベルは、タイムラインの行から離れる方向（画面上は常に上方向）に一定間隔（12px）でスタッキング（レベル分け）してオフセットする。
3. スタッキングされたラベル（レベル > 0）には、ドット/バーと接続する細い破線（`.tdsl-label-leader`）が自動で追加される。

horizontal / vertical の両 orientation で同様に動作します。重なりがない（少数・疢でない）場合はすべてのラベルがレベル0（オフセットなし）で、回帰はありません。

#### 同一レーン内で重なる Span/EventRange のサブ行スタッキング（#549）

同一レーン内で期間が重複する `span` / `event_range` は、バー本体が完全に重ならないよう自動的にサブ行へ振り分けられます。

1. レーンごとに、`span` / `event_range` を開始時刻順に走査し、区間スケジューリング（greedy interval coloring）でサブ行（レベル）を割り当てる。重ならないアイテムは同じレベル（レベル0）を共有できる。
2. レベル > 0 のアイテムは、クロス軸方向（horizontal ではY、vertical ではX）にレベル数 × 40px（`lane_height` に応じて #507 のdensityスケーリングと同率で拡大/縮小）だけオフセットされる。
3. サブ行が必要になったレーンは、実効高さ（horizontal）/実効幅（vertical）が必要なレベル数だけ自動拡張される。重なりがないレーンは `lane_height` のまま変化せず、回帰はない。
4. `--layout-style group-bands` の背景帯・レーン背景帯（`.tdsl-lane-band-*`）は拡張後の実効サイズに追従する。

イベントラベルの衝突回避（#537、上記）とは独立した仕組みで、バー本体（`.tdsl-span` / `.tdsl-event-range`）の重なりを防ぎます。

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

### 継続中（open-ended）アイテム（#550）

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-item-open-ended` | `end` に `now` を指定した span/event_range の `<g>` に付与されるフッククラス | デフォルトでは内側の `.tdsl-span` / `.tdsl-event-range` の `stroke-dasharray` を `4 2`（破線）に上書き |

```css
/* 継続中アイテムのデフォルトの破線囲みを矢印風に変える例 */
.tdsl-item-open-ended .tdsl-span,
.tdsl-item-open-ended .tdsl-event-range {
  stroke-dasharray: none;
  opacity: 0.85;
}
```

### ツールチップ

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-tooltip` | ホバー時に表示されるツールチップ | `background`, `border`, `border-radius`（デフォルト: 6px）, `color`, `font-size`, `box-shadow` |

### インタラクティブモード限定（`--interactive`）

`--interactive` を指定した場合のみ出力される凡例パネルの絞り込みトグル用クラスです（issue #755）。lane トグルは #tdsl-legend 内に元から存在しますが、タグトグルは `timeline.color_map` にタグが登録されている場合のみ出力されます。

| クラス | 対象 | 主なプロパティ |
|---|---|---|
| `.tdsl-legend-item` | lane / タグ絞り込みチェックボックスの行 | `display`, `gap`, `padding`, `cursor` |
| `.tdsl-legend-section-title` | 凡例パネル内の「タグ」セクション見出し | `font-weight`, `color`, `font-size` |
| `.tdsl-legend-swatch` | タグ絞り込み行の色スウォッチ（`color_map` の色を反映） | `background`（インラインスタイルで設定）, `width`, `height`, `border-radius` |
| `.tdsl-lane-hidden` | lane トグルで非表示にした `.tdsl-item` に付与 | `display: none` |
| `.tdsl-tag-hidden` | タグトグルで非表示にした `.tdsl-item` に付与 | `display: none` |

lane トグルとタグトグルは AND で合成されます（`.tdsl-lane-hidden` と `.tdsl-tag-hidden` のどちらか一方でも付けば非表示）。タグトグルは OR セマンティクス（チェックしたタグを1つも持たない item を隠す）で、全チェックを外すと絞り込み自体が無効化されます（全 item 表示に戻る）。

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

## アクセシビリティ方針

`tdsl render` が出力するSVGは、スクリーンリーダー等の支援技術に対応するための ARIA 属性を付与しています。

### SVGルート

```xml
<svg ... role="img" aria-label="timeline" ...>
```

SVG 全体を `role="img"` として宣言し、`aria-label="timeline"` でコンテンツの目的を伝えます。

### アイテム要素（span / event / event_range）

各アイテムの `<g>` 要素に `role="group"` と `aria-label` を付与します。

```xml
<g class="tdsl-item tdsl-item-span"
   role="group"
   aria-label="Span: 漢, BC206〜220, source: wd:Q7209, id: span:han, Lane: 漢"
   tabindex="0" ...>
```

`aria-label` の形式: `"<種別>: <ツールチップ情報（改行を「, 」に変換）>, Lane: <レーン名>"`

- 種別は `Span` / `Event` / `Event range` のいずれか（#701: 周辺ツール群（`obsidian-tdsl` 等）のUI言語との一貫性のため英語固定。以前は日本語ハードコードだった）
- ツールチップの改行は `, ` に変換されて 1 行に収められる
- `<title>` 要素はブラウザのツールチップ表示用のフォールバックとして残す

### 装飾要素

スクリーンリーダーに読み上げさせる必要のない装飾要素には `role="presentation"` および `aria-hidden="true"` を付与します。

| 要素 | 付与属性 |
|---|---|
| レーン帯背景 `<rect class="tdsl-lane-band-*">` | `role="presentation" aria-hidden="true"` |
| 時間軸ベースライン `<line class="tdsl-axis-baseline">` | `role="presentation"` |
| 目盛り線 `<line class="tdsl-axis-tick">` | `role="presentation"` |
| 月目盛り線 `<line class="tdsl-axis-month-tick">` | `role="presentation"` |
| 日目盛り線 `<line class="tdsl-axis-day-tick">` | `role="presentation"` |
| グリッド線 `<line class="tdsl-grid-line">` | `role="presentation"` |
| グループ区切り線 `<line class="tdsl-group-separator">` | `role="presentation"` |

軸ラベルテキスト（`<text class="tdsl-axis-text">`）やレーン名テキスト（`<text class="tdsl-lane-label">`）は意味のある内容のため、これらには `role="presentation"` を付与しません。

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
