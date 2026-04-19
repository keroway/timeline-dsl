# スプリント計画 2026-04-20

## 今スプリントの目標

**「tdsl を誰でも使えるツールにする」**

配布インフラの整備（Issue #34）と描画 UX の改善（Issue #35）を並行実施し、
エンドユーザーへのアダプションを飛躍的に高める。

---

## スコープ一覧

| Issue | タイトル | 担当領域 | 優先度 |
|-------|---------|---------|--------|
| #34 | CLIバイナリのリリース配布（install.sh / GitHub Releases） | BE/CLI | 最優先 |
| #35 | SVGレンダリングのリッチ化（スケール調整・配色テーマ・カスタムCSS） | Render/CLI | 並行 |
| #28 | Wikidata取得キャッシュ（TTL / オフライン連携） | BE | バックログ |
| #36 | WebUI編集画面 | FE | 将来スプリント（スコープ外） |

---

## BE/CLIエンジニア向け: Issue #34（リリース配布）

### 概要

`tdsl` CLI バイナリを GitHub Releases で公開し、`curl URL | sh` でインストールできる仕組みを構築する。

### タスク一覧

#### タスク 1: GitHub Actions ワークフローの作成

ファイル: `.github/workflows/release.yml`

```yaml
# トリガー
on:
  push:
    tags:
      - 'v*'

# ビルドマトリクス（最低限）
jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
            archive: tdsl-linux-x86_64.tar.gz
          - target: x86_64-apple-darwin
            os: macos-latest
            archive: tdsl-macos-x86_64.tar.gz
          - target: aarch64-apple-darwin
            os: macos-latest
            archive: tdsl-macos-aarch64.tar.gz
```

- `x86_64-unknown-linux-musl` は静的リンクのために `musl-tools` と `cross` を使用
- バイナリを tar.gz に圧縮して `gh release upload` でアセットとしてアップロード
- `softprops/action-gh-release` アクションを活用するとリリースページ作成が簡略化できる

#### タスク 2: install.sh の作成

ファイル: `install.sh`（プロジェクトルートに配置）

```sh
#!/bin/sh
set -eu

REPO="keroway/timeline-dsl"
BIN_DIR="${HOME}/.local/bin"
BIN_NAME="tdsl"

# --- OS・アーキテクチャ判定 ---
OS=$(uname -s)
ARCH=$(uname -m)

case "${OS}-${ARCH}" in
  Linux-x86_64)  ARCHIVE="tdsl-linux-x86_64.tar.gz" ;;
  Darwin-x86_64) ARCHIVE="tdsl-macos-x86_64.tar.gz" ;;
  Darwin-arm64)  ARCHIVE="tdsl-macos-aarch64.tar.gz" ;;
  *)
    echo "Unsupported platform: ${OS}-${ARCH}" >&2
    exit 1
    ;;
esac

# --- バージョン解決 ---
if [ -z "${TDSL_VERSION:-}" ]; then
  TDSL_VERSION=$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | sed 's/.*"tag_name": *"\(.*\)".*/\1/')
fi

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TDSL_VERSION}/${ARCHIVE}"

# --- ダウンロード・展開・配置 ---
TMP=$(mktemp -d)
trap "rm -rf ${TMP}" EXIT

curl -sSfL "${DOWNLOAD_URL}" | tar -xz -C "${TMP}"
mkdir -p "${BIN_DIR}"
mv "${TMP}/${BIN_NAME}" "${BIN_DIR}/${BIN_NAME}"
chmod +x "${BIN_DIR}/${BIN_NAME}"

echo "Installed ${BIN_NAME} ${TDSL_VERSION} to ${BIN_DIR}/${BIN_NAME}"
```

実装上の注意:
- `--version v0.1.0` オプション（環境変数 `TDSL_VERSION` 経由）をサポート
- `~/.local/bin` が `PATH` にない場合、`.bashrc` / `.zshrc` への追記を案内するメッセージを出力
- Windows は PowerShell スクリプト（`install.ps1`）を別途作成することを推奨（今スプリントは任意）

#### タスク 3: バージョン整備

- `Cargo.toml`（workspace）の `version = "0.1.0"` を確認・固定
- `tdsl --version` コマンドの出力を確認
- `CHANGELOG.md` を新規作成（プロジェクトルート）し、v0.1.0 の変更内容を記載

#### タスク 4: README 更新

`README.md` にインストールセクションを追加:

```markdown
## インストール

### curl でワンラインインストール（macOS / Linux）

\`\`\`sh
curl -sSfL https://raw.githubusercontent.com/keroway/timeline-dsl/main/install.sh | sh
\`\`\`

インストール後:

\`\`\`sh
tdsl --version
\`\`\`

### cargo からインストール（Rustユーザー向け）

\`\`\`sh
cargo install --git https://github.com/keroway/timeline-dsl tdsl-cli
\`\`\`
```

### 受け入れ条件チェックリスト

- [ ] `.github/workflows/release.yml` が `v*` タグで起動し、3プラットフォームのバイナリをリリースアセットとして公開する
- [ ] `install.sh` が curl パイプで動作する（macOS Apple Silicon でのローカル動作確認必須）
- [ ] `tdsl --version` が `tdsl 0.1.0` を出力する
- [ ] `README.md` にインストール手順が追記される
- [ ] `v0.1.0` タグをプッシュして実際の GitHub Releases ページを確認する

---

## Renderエンジニア（FE/Render）向け: Issue #35（レンダリングリッチ化）

### 概要

`tdsl render` コマンドで生成する HTML タイムラインの視覚表現を拡充する。
CLIオプション追加・カラーテーマ・カスタムCSS注入・レーン別カラーの4点を実装する。

### 対象ファイル

- `crates/tdsl-render/src/layout.rs` — `RenderOptions` 拡張
- `crates/tdsl-render/src/html.rs` — テーマCSS追加、カスタムCSS注入
- `crates/tdsl-render/src/svg.rs` — レーン別カラー割り当て
- `crates/tdsl-cli/src/main.rs` — CLI オプション追加

### タスク一覧

#### タスク 1: RenderOptions の拡張

`crates/tdsl-render/src/layout.rs` の `RenderOptions` に以下を追加:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Default,
    Dark,
    Print,
    Pastel,
}

pub struct RenderOptions {
    pub scale: f64,
    pub lane_height: f64,
    pub left_gutter: f64,
    pub top_margin: f64,
    pub right_margin: f64,
    pub bottom_margin: f64,
    // 新規追加フィールド
    pub theme: Theme,
    pub custom_css: Option<String>,  // CSSファイルの内容（ファイルパスではなく内容）
}
```

#### タスク 2: カラーテーマCSSの実装

`crates/tdsl-render/src/html.rs` で以下を実装:

```rust
const THEME_CSS_DARK: &str = r#"
body { background: #1a1a2e; color: #e0e0e0; }
.tdsl-timeline { background: #16213e; border-color: #0f3460; }
.tdsl-lane-band-even { fill: #16213e; }
.tdsl-lane-band-odd  { fill: #0f3460; }
/* ... */
"#;

const THEME_CSS_PRINT: &str = r#"
/* モノクロ高コントラスト */
.tdsl-span { fill: #000; stroke: #000; }
/* ... */
"#;

const THEME_CSS_PASTEL: &str = r#"
/* パステルカラー */
"#;
```

`wrap_html` のシグネチャを `wrap_html(svg_body: &str, title: &str, opts: &RenderOptions) -> String` に変更し、
`opts.theme` に応じて追加CSSを注入する。

`opts.custom_css` が `Some(css)` の場合、テーマCSSの後にさらに追記する。

#### タスク 3: レーン別カラーパレット

`crates/tdsl-render/src/svg.rs` でレーン別カラーを自動割り当て:

```rust
/// 視認性の高い8色パレット（colorblind-friendly を優先）
const LANE_PALETTE: &[&str] = &[
    "#4682B4", // steel blue（現行デフォルト）
    "#E67E22", // orange
    "#27AE60", // green
    "#8E44AD", // purple
    "#E74C3C", // red
    "#1ABC9C", // teal
    "#F39C12", // yellow-orange
    "#2980B9", // darker blue
];

fn lane_color(lane_index: usize) -> &'static str {
    LANE_PALETTE[lane_index % LANE_PALETTE.len()]
}
```

`LayoutModel` に `lane_color_map: HashMap<String, &'static str>` を追加し、
`LaidItem::Span` の `<rect>` に `fill` 属性として直接指定する（CSSクラスのオーバーライド）。

#### タスク 4: CLI オプション追加

`crates/tdsl-cli/src/main.rs` の `Render` variant に追加:

```rust
Render {
    input: PathBuf,
    output: Option<PathBuf>,

    #[arg(long, default_value_t = 2.0)]
    scale: f64,

    // 新規追加
    #[arg(long, default_value_t = 60.0)]
    lane_height: f64,

    #[arg(long, default_value_t = 120.0)]
    left_gutter: f64,

    #[arg(long, default_value_t = 40.0)]
    top_margin: f64,

    #[arg(long, default_value = "default", value_enum)]
    theme: ThemeArg,

    #[arg(long)]
    custom_css: Option<PathBuf>,

    #[arg(long, default_value_t = false)]
    offline: bool,
}
```

`ThemeArg` は clap の `ValueEnum` を derive した enum を定義し、`RenderOptions::Theme` に変換する。

#### タスク 5: テスト追加

`crates/tdsl-render/src/html.rs` のテストに追加:

```rust
#[test]
fn dark_theme_css_is_injected() {
    let opts = RenderOptions { theme: Theme::Dark, ..Default::default() };
    let html = wrap_html("<svg></svg>", "test", &opts);
    assert!(html.contains("1a1a2e")); // dark background color
}

#[test]
fn custom_css_is_appended_after_theme() {
    let opts = RenderOptions {
        custom_css: Some(".tdsl-span { fill: hotpink; }".into()),
        ..Default::default()
    };
    let html = wrap_html("<svg></svg>", "test", &opts);
    assert!(html.contains("hotpink"));
}
```

### 受け入れ条件チェックリスト

- [ ] `--lane-height`, `--left-gutter`, `--top-margin` オプションが `tdsl render` に追加される
- [ ] `--theme dark|print|pastel|default` で異なるCSSが埋め込まれる
- [ ] `--custom-css my.css` でファイル内容がHTMLに追記される（ファイル不在時はエラー終了）
- [ ] laneごとに異なる色が自動割り当てされる（同一lane は常に同一色）
- [ ] 既存の全テスト（`cargo test -p tdsl-render`）が通過する
- [ ] `README.md` の render コマンド例に新オプションの使用例が追記される

---

## スコープ外の判断とその根拠

### Issue #36（WebUI編集画面）を今スプリントのスコープ外とした理由

1. **配布インフラ未整備**: WebUI はバックエンドに tdsl-core が必要だが、そのバイナリ配布（#34）が未完。
   先にライブラリとしての tdsl-core の境界を整理してから WebUI 実装に入るべき。

2. **技術選定の前提が未確定**: wasm か HTTP API か、フロントエンドのフレームワーク、ホスティング先など、
   1スプリント以内で決定するには情報が足りない。設計スプリントを1本挟む必要がある。

3. **作業量が大きい**: ファイルアップロード・エディタ・リアルタイムプレビュー・保存で最低2〜3スプリント分。
   中途半端な実装はユーザー体験を損なうため、一気通貫で実装できるスプリントで取り組む。

4. **#35 が前提**: WebUI のプレビュー品質は #35 の配色・テーマ機能に依存する。

**推奨順序**: #34（リリース配布）→ #35（レンダリング）→ #28（キャッシュ）→ #36（WebUI）

---

## タイムライン（目安）

| 日程 | マイルストーン |
|------|--------------|
| 04/20（本日） | issue 作成・計画確定・作業開始 |
| 04/21〜22 | #34 GitHub Actions ワークフロー & install.sh 実装 |
| 04/21〜22 | #35 RenderOptions 拡張 & テーマCSS実装 |
| 04/23 | #34 v0.1.0 タグによる動作確認・README 更新 |
| 04/23 | #35 CLI オプション追加・テスト通過確認 |
| 04/24 | PR レビュー・マージ |
| 04/24 | スプリントレビュー（`sprint-review-2026-04-20.md` に追記） |

---

## 注意事項

- `cargo build --release` のバイナリサイズが大きい場合（>10MB）は `strip` と `opt-level = "z"` を検討する
- GitHub Actions の macOS runner は高価なため、タグプッシュのみにトリガーを絞ること
- `install.sh` は POSIX sh（`/bin/sh`）で動作すること（bash 依存禁止）
- CSS の `Theme::Dark` は `prefers-color-scheme: dark` メディアクエリと組み合わせる拡張も将来検討する
