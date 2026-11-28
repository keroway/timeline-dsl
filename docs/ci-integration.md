# GitHub Actions 連携ガイド

Timeline DSL は GitHub Actions composite action として公開されており、CI/CD パイプラインで `.tdsl` ファイルを SVG / HTML / PNG / PDF にレンダリングできます。

## 参照する版

`uses:` には完全な版番号のタグ（例: `keroway/timeline-dsl@v3.0.1`）を指定します。`@v3` のようなメジャー版だけの移動タグは提供していません。新しい版は [Releases](https://github.com/keroway/timeline-dsl/releases) で確認できます。

`uses:` のタグが固定するのは action の定義です。ダウンロードする tdsl CLI の版は `version` 入力で別に決まり、既定は `latest` です。

## クイックスタート

```yaml
# .github/workflows/render-timeline.yml
name: Render Timeline

on:
  push:
    paths: ['**/*.tdsl']

jobs:
  render:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: keroway/timeline-dsl@v3.0.1
        with:
          file: examples/china_dynasties.tdsl
          format: svg
          output: output/china.svg

      - uses: actions/upload-artifact@v4
        with:
          name: timeline-svg
          path: output/
```

## インプット

| インプット | 必須 | デフォルト | 説明 |
|---|---|---|---|
| `file` | ✅ | — | レンダリングする `.tdsl` ファイルのパス |
| `format` | — | `svg` | 出力フォーマット: `svg` / `html` / `png` / `pdf` |
| `output` | — | `<basename>.<format>` | 出力ファイルパス |
| `offline` | — | `false` | オフラインモード（Wikidata フェッチをスキップ） |
| `interactive` | — | `false` | インタラクティブ HTML 出力（`format: html` 時のみ有効） |
| `theme` | — | （CLI デフォルト） | テーマ: `default` / `dark` / `print` / `pastel` |
| `scale` | — | （CLI デフォルト） | 水平軸のピクセル/年レート |
| `version` | — | `latest` | 使用する tdsl バージョン（例: `v3.0.1`） |

## アウトプット

| アウトプット | 説明 |
|---|---|
| `output_path` | 生成された出力ファイルの絶対パス |

## ユースケース別レシピ

### PR で変更された .tdsl ファイルを SVG プレビューとして添付

action の `file` は1ファイルだけを受け取ります。変更された `.tdsl` の一覧を JSON 配列で出力し、`matrix` で1ファイルずつレンダリングします。

```yaml
name: TDSL Preview

on:
  pull_request:
    paths: ['**/*.tdsl']

permissions:
  contents: read

jobs:
  changed:
    runs-on: ubuntu-latest
    outputs:
      files: ${{ steps.changed.outputs.files }}
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: List changed .tdsl files
        id: changed
        env:
          BASE_REF: ${{ github.base_ref }}
        run: |
          files=$(git diff --name-only -z --diff-filter=d "origin/$BASE_REF...HEAD" -- '*.tdsl' \
            | jq -Rsc 'split("\u0000") | map(select(length > 0))')
          echo "files=$files" >> "$GITHUB_OUTPUT"

  preview:
    needs: changed
    if: needs.changed.outputs.files != '[]'
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        file: ${{ fromJSON(needs.changed.outputs.files) }}
    steps:
      - uses: actions/checkout@v4

      - uses: keroway/timeline-dsl@v3.0.1
        id: render
        with:
          file: ${{ matrix.file }}
          format: svg
          offline: 'true'

      - uses: actions/upload-artifact@v4
        with:
          name: tdsl-preview-${{ strategy.job-index }}
          path: ${{ steps.render.outputs.output_path }}
```

- `--diff-filter=d` で削除されたファイルを除きます。削除だけの PR では一覧が `[]` になり、`preview` ジョブは実行されません。
- ブランチ名は `env:` 経由で渡します。`run:` の中で `${{ github.base_ref }}` を直接展開すると、ブランチ名がシェルの構文として解釈されます。
- artifact 名は `matrix` のジョブごとに分けています。同じ名前で複数回アップロードすると失敗します。

### インタラクティブ HTML を GitHub Pages にデプロイ

```yaml
name: Deploy Timeline

on:
  push:
    branches: [main]
    paths: ['timelines/**/*.tdsl']

jobs:
  deploy:
    runs-on: ubuntu-latest
    permissions:
      pages: write
      id-token: write
    steps:
      - uses: actions/checkout@v4

      - uses: keroway/timeline-dsl@v3.0.1
        with:
          file: timelines/main.tdsl
          format: html
          output: public/index.html
          interactive: 'true'
          offline: 'true'

      - uses: actions/upload-pages-artifact@v3
        with:
          path: public/

      - uses: actions/deploy-pages@v4
```

### ダークテーマで SVG を生成してリリースに添付

```yaml
- uses: keroway/timeline-dsl@v3.0.1
  id: render
  with:
    file: timeline.tdsl
    format: svg
    output: timeline-dark.svg
    theme: dark

- name: Upload to release
  uses: softprops/action-gh-release@v2
  with:
    files: ${{ steps.render.outputs.output_path }}
```

### PNG 画像としてレンダリングしてリリースに添付

```yaml
- uses: keroway/timeline-dsl@v3.0.1
  id: render
  with:
    file: timeline.tdsl
    format: png
    output: timeline.png

- name: Upload to release
  uses: softprops/action-gh-release@v2
  with:
    files: ${{ steps.render.outputs.output_path }}
```

### 特定バージョンを固定して使用

```yaml
- uses: keroway/timeline-dsl@v3.0.1
  with:
    file: timeline.tdsl
    version: v3.0.1   # CLI のバージョンを固定
```

## 対応プラットフォーム

| OS | アーキテクチャ | サポート |
|---|---|---|
| Ubuntu / Linux | x86_64 | ✅ |
| Ubuntu / Linux | ARM64 | ✅ |
| macOS | x86_64 (Intel) | ✅ |
| macOS | ARM64 (Apple Silicon) | ✅ |
| Windows | x86_64 | ✅ |

## 注意事項

- **Wikidata フェッチ**: `import wikidata` を使う `.tdsl` ファイルは `offline: 'true'` でなければ Wikidata API を呼び出します。CI での繰り返し実行には `offline: 'true'` + ローカルキャッシュの事前投入（`tdsl cache` コマンド）を推奨します。
- **バージョン固定**: 本番 CI では `version: vX.Y.Z` で固定することを推奨します。`latest` は常に最新版を取得します。
- **GitHub Actions のキャッシュ**: tdsl バイナリ自体は毎回ダウンロードします。ダウンロード時間を削減するには `actions/cache` でバイナリをキャッシュする構成も可能です。

## 関連ドキュメント

- [DSL 言語仕様](dsl-spec.md)
- [スタイルカスタマイズガイド](styling.md)
- [Getting Started チュートリアル](tutorial.md)
