# スプリント計画 2026-04-21

## 今スプリントの目標

**「Wikidata取得のキャッシュ機能を追加し、開発サイクルを高速化する」**

前スプリント（2026-04-20）で配布インフラ（#34）とレンダリングリッチ化（#35）を完了させた。
今スプリントはバックログから #28（Wikidataキャッシュ）を取り上げ、
開発者体験（DX）の向上を図る。

---

## 人員配置

| 役割 | 担当者 | 担当領域 |
|------|--------|---------|
| プロジェクトマネージャー | 山田 | スプリント計画・設計レビュー・issue管理・進捗調整 |
| バックエンドエンジニア（Wikidata） | 佐藤 | `tdsl-wikidata` キャッシュ層の実装（デコレータパターン、TTL管理、ファイルI/O） |
| バックエンドエンジニア（CLI/統合） | 田中 | `tdsl-cli` CLIオプション追加・`load_ir` 統合・統合テスト |

### 役割詳細

#### 山田（マネージャー）
- スプリント計画書・レビュードキュメントの作成
- 設計方針の承認（デコレータパターン vs. 内蔵キャッシュ）
- 受け入れ条件の管理と最終確認
- issueへの実施結果コメント

#### 佐藤（エンジニア1 / Wikidataキャッシュ層）
- `crates/tdsl-wikidata/src/cache.rs` の新規作成
- `CachedWikidataClient<C: WikidataClient>` 構造体の実装
- TTLベースのキャッシュヒット判定（ファイルの更新時刻を利用）
- キャッシュファイルのパス管理（`~/.cache/tdsl/<cache_key>.json`）
- `tdsl-wikidata/Cargo.toml` の依存追加（`dirs`クレート）
- ユニットテストの作成

#### 田中（エンジニア2 / CLI統合）
- `tdsl-cli/src/main.rs` への `--no-cache` / `--cache-ttl` オプション追加
- `load_ir` 関数のシグネチャ更新とキャッシュ付きクライアントの組み込み
- `build` / `check` / `render` / `ast` コマンドへのキャッシュオプション伝播
- 統合テストおよび既存テストの通過確認

---

## スコープ

| Issue | タイトル | 担当 | 優先度 |
|-------|---------|------|--------|
| #28 | Wikidata取得キャッシュ（TTL / オフライン連携） | 佐藤・田中 | **最優先** |

---

## 設計方針（マネージャー承認済み）

### キャッシュ層のアーキテクチャ

**デコレータパターン** を採用する。`WikidataClient` trait を実装した `CachedWikidataClient<C>` が
内部クライアント `C` をラップし、キャッシュヒット時はファイルから読み込む。

```
CachedWikidataClient<HttpWikidataClient>
  ├── キャッシュチェック
  │     hit  → ファイルから WikidataEntity を返す
  │     miss → inner.get_entity() を呼び出してファイルに保存
  └── TTL超過時はキャッシュを無視して再取得
```

### キャッシュファイルの設計

- **パス**: `~/.cache/tdsl/<cache_key>.json`
- **キャッシュキー**:
  - QIDベース: `get_<qid>_<langs_joined_by_dash>.json`（例: `get_Q7209_ja-en.json`）
  - サイトリンクベース: `sitelink_<site>_<url_encoded_title>_<langs_joined_by_dash>.json`
- **TTL**: デフォルト 86400秒（24時間）。ファイルの `mtime` と比較して判定。
- **対象**: `get_entity` / `get_entity_by_sitelink` のみキャッシュ対象。
  `search_entities` / `sparql_query` はキャッシュしない（結果が動的に変わりやすいため）。

### CLI オプション

| オプション | デフォルト | 説明 |
|-----------|----------|------|
| `--no-cache` | false | キャッシュを無視して強制再取得 |
| `--cache-ttl <seconds>` | 86400 | キャッシュ有効期間（秒）。0 で無効化（`--no-cache` 相当） |

適用コマンド: `build`, `check`, `render`, `ast`（`load_ir` を通じて適用）

### `load_ir` の変更

```rust
// 変更前
fn load_ir(input: &Path, offline: bool) -> Result<TimelineIr, String>

// 変更後
fn load_ir(input: &Path, offline: bool, cache_opts: CacheOptions) -> Result<TimelineIr, String>

pub struct CacheOptions {
    pub no_cache: bool,
    pub ttl_secs: u64,  // デフォルト 86400
}
```

---

## タスク一覧

### エンジニア1（佐藤）: キャッシュ層実装

1. `crates/tdsl-wikidata/Cargo.toml` に `dirs = "5"` を追加
2. `crates/tdsl-wikidata/src/cache.rs` を新規作成
   - `CacheOptions` 構造体（`no_cache: bool`, `ttl: Duration`）
   - `CachedWikidataClient<C>` 構造体
   - `WikidataClient` trait の実装（`get_entity`, `get_entity_by_sitelink`, `search_entities`, `sparql_query`）
   - `get_entity` / `get_entity_by_sitelink` のキャッシュロジック
3. `crates/tdsl-wikidata/src/lib.rs` に `pub mod cache;` を追加し型をre-export
4. テスト作成（TTLヒット/ミス/失効の各ケース）

### エンジニア2（田中）: CLI統合

5. `crates/tdsl-cli/src/main.rs` の `load_ir` を `CacheOptions` を受け取る形に変更
6. `load_ir` 内で `CachedWikidataClient` を使用するよう更新
7. `Build`, `Check`, `Render`, `Ast` subcommand に `--no-cache`, `--cache-ttl` オプション追加
8. 既存テストが通過することを確認

---

## 受け入れ条件（Issue #28 より）

- [ ] キャッシュヒット時に API リクエストが発生しないこと
- [ ] `--no-cache` で強制再取得できること
- [ ] キャッシュ失効（TTL超過）時は自動的に再取得すること
- [ ] `cargo test --workspace` が全テスト通過すること

---

## タイムライン

| 日程 | マイルストーン |
|------|--------------|
| 04/21（本日） | 計画確定・作業開始 |
| 04/21 | キャッシュ層（`cache.rs`）実装完了 |
| 04/21 | CLI統合・オプション追加完了 |
| 04/21 | テスト通過確認・issue クローズ |
| 04/21 | スプリントレビュー（`sprint-review-2026-04-21.md` 作成） |

---

## 注意事項

- キャッシュディレクトリ（`~/.cache/tdsl/`）が存在しない場合は自動生成する
- `dirs` クレートで `cache_dir()` が `None` を返す環境（一部CI等）では、キャッシュを無効化してフォールバックする
- `get_entity_by_sitelink` のキャッシュキーにタイトルを含める際は URL エンコードを使用してファイル名の安全性を確保する
- Windows 対応: `dirs::cache_dir()` は Windows では `%LOCALAPPDATA%\cache\tdsl` 相当になる
