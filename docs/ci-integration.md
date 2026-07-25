# GitHub Actions 連携ガイド

Timeline DSL は GitHub Actions composite action として公開されており、CI/CD パイプラインで `.tdsl` ファイルを SVG または HTML にレンダリングできます。

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

      - uses: keroway/timeline-dsl@v1
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
| `format` | — | `svg` | 出力フォーマット: `svg` または `html` |
| `output` | — | `<basename>.<format>` | 出力ファイルパス |
| `offline` | — | `false` | オフラインモード（Wikidata フェッチをスキップ） |
| `interactive` | — | `false` | インタラクティブ HTML 出力（`format: html` 時のみ有効） |
| `theme` | — | （CLI デフォルト） | テーマ: `default` / `dark` / `print` / `pastel` |
| `scale` | — | （CLI デフォルト） | 水平軸のピクセル/年レート |
| `version` | — | `latest` | 使用する tdsl バージョン（例: `v1.27.0`） |

## アウトプット

| アウトプット | 説明 |
|---|---|
| `output_path` | 生成された出力ファイルの絶対パス |

## ユースケース別レシピ

### PR で変更された .tdsl ファイルを SVG プレビューとして添付

```yaml
name: TDSL Preview

on:
  pull_request:
    paths: ['**/*.tdsl']

jobs:
  preview:
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Get changed .tdsl files
        id: changed
        run: |
          FILES=$(git diff --name-only "origin/${{ github.base_ref }}...HEAD" | grep '\.tdsl$' || true)
          echo "files=$FILES" >> $GITHUB_OUTPUT

      - uses: keroway/timeline-dsl@v1
        if: steps.changed.outputs.files != ''
        with:
          file: ${{ steps.changed.outputs.files }}
          format: svg
          offline: 'true'

      - uses: actions/upload-artifact@v4
        with:
          name: tdsl-preview
          path: '*.svg'
```

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

      - uses: keroway/timeline-dsl@v1
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
- uses: keroway/timeline-dsl@v1
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

### 特定バージョンを固定して使用

```yaml
- uses: keroway/timeline-dsl@v1
  with:
    file: timeline.tdsl
    version: v1.27.0   # バージョンを固定
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
