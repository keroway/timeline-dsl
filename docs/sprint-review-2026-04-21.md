# スプリントレビュー 2026-04-21

## 全体方針（マネージャー視点）

前スプリント（2026-04-20）で配布インフラ（#34）とレンダリングリッチ化（#35）を完了させた。
今スプリントはバックログから #28（Wikidata キャッシュ）を取り上げ、
開発者体験（DX）の向上を達成した。

---

## 人員配置

| 役割 | 担当者 | 担当領域 |
|------|--------|---------|
| プロジェクトマネージャー | 山田 | スプリント計画・設計レビュー・issue管理 |
| バックエンドエンジニア（Wikidata） | 佐藤 | `tdsl-wikidata` キャッシュ層の実装 |
| バックエンドエンジニア（CLI/統合） | 田中 | `tdsl-cli` CLIオプション追加・統合 |

---

## 今スプリントの成果

| Issue | タイトル | 担当 | 結果 |
|-------|---------|------|------|
| #28 | Wikidata取得キャッシュ（TTL / オフライン連携） | 佐藤・田中 | **完了** |

---

## エンジニア実装結果

### Issue #28: Wikidata取得キャッシュ

**実装完了**

#### エンジニア1（佐藤）: キャッシュ層（`tdsl-wikidata`）

- `crates/tdsl-wikidata/src/cache.rs` を新規作成
  - `CacheOptions` 構造体（`no_cache: bool`, `ttl: Duration`）
    - デフォルト TTL: 86400秒（24時間）
  - `CachedWikidataClient<C>` 構造体（デコレータパターン）
    - `get_entity` / `get_entity_by_sitelink` のみキャッシュ対象
    - `search_entities` / `sparql_query` は常に API を呼び出す
  - キャッシュファイルの命名規則:
    - QIDベース: `~/.cache/tdsl/get_<QID>_<langs>.json`
    - サイトリンクベース: `~/.cache/tdsl/sitelink_<site>_<safe_title>_<langs>.json`
  - TTL チェック: ファイルの `mtime` と `SystemTime::now()` を比較
  - `dirs::cache_dir()` が `None` の環境では `temp_dir()/tdsl_cache` にフォールバック
  - キャッシュ書き込みエラーは警告を `eprintln!` で出力して継続（非致命的）
- `dirs = "5"` を `crates/tdsl-wikidata/Cargo.toml` に追加
- `tempfile = "3"` を dev-dependencies に追加
- `crates/tdsl-wikidata/src/lib.rs` に `pub mod cache` と `CacheOptions`・`CachedWikidataClient` を re-export

#### エンジニア2（田中）: CLI統合（`tdsl-cli`）

- `load_ir` のシグネチャを変更:
  ```rust
  // 変更前
  fn load_ir(input: &Path, offline: bool) -> Result<TimelineIr, String>
  // 変更後
  fn load_ir(input: &Path, offline: bool, cache_opts: CacheOptions) -> Result<TimelineIr, String>
  ```
- `build` コマンドに `--no-cache` / `--cache-ttl <seconds>` オプションを追加
- `render` コマンドに `--no-cache` / `--cache-ttl <seconds>` オプションを追加

#### 使用例

```bash
# 通常ビルド（キャッシュあり、TTL=24h）
tdsl build examples/china_with_import.tdsl --pretty

# キャッシュを無視して強制再取得
tdsl build examples/china_with_import.tdsl --no-cache --pretty

# TTLを1時間に設定
tdsl build examples/china_with_import.tdsl --cache-ttl 3600 --pretty

# キャッシュ無効化（TTL=0）
tdsl build examples/china_with_import.tdsl --cache-ttl 0 --pretty
```

---

## テスト結果

**全72テスト通過**（前スプリントの67 + キャッシュ関連 5件追加）

| テスト | 結果 |
|--------|------|
| `cache::tests::cache_hit_avoids_api_call` | ✅ |
| `cache::tests::no_cache_forces_api_call` | ✅ |
| `cache::tests::expired_cache_triggers_api_call` | ✅ |
| `cache::tests::sitelink_cache_hit_avoids_api_call` | ✅ |
| `cache::tests::different_langs_use_different_cache_files` | ✅ |
| 既存テスト（67件） | ✅ |

---

## 受け入れ条件チェックリスト（Issue #28）

- [x] キャッシュヒット時に API リクエストが発生しないこと
- [x] `--no-cache` で強制再取得できること
- [x] キャッシュ失効（TTL超過）時は自動的に再取得すること
- [x] `cargo test --workspace` が全テスト通過すること

---

## アーキテクチャ上の判断

### デコレータパターンの採用

`WikidataClient` trait をそのまま実装した `CachedWikidataClient<C>` を採用した。
これにより:
- テスト時にモッククライアントをキャッシュ層でラップして動作確認できる
- `HttpWikidataClient` の変更なしにキャッシュ機能を後付けできる
- 将来的に別のキャッシュバックエンド（Redis等）を差し込む拡張も容易

### キャッシュ対象の選別

`search_entities` と `sparql_query` はキャッシュしない判断をした。
これは動的な結果（Wikidata の更新に連動するべき）を扱うためである。
エンティティ本体（`get_entity` / `get_entity_by_sitelink`）は静的な構造変化が少なく、
キャッシュが有効に働く。

### エラー処理方針

キャッシュの読み書きエラーは致命的エラーとせず、警告を出力してフォールバックする設計にした。
キャッシュはあくまで補助機能であり、API 呼び出しが成功すれば処理を続行できるため。

---

## テクニカルリスクと対策

### キャッシュファイルの肥大化

- **リスク**: 長期運用で `~/.cache/tdsl/` が大量のファイルで埋まる可能性
- **対策**: 現バージョンは手動削除（`rm -rf ~/.cache/tdsl/`）で対応。
  将来的に `tdsl cache clear` サブコマンドを追加検討

### 並行実行時のキャッシュ競合

- **リスク**: 複数の `tdsl` プロセスが同一キャッシュファイルに同時書き込みした場合、
  JSON が壊れる可能性
- **対策**: 現バージョンでは対応なし（シングルプロセス運用が前提）。
  将来的にアトミック書き込み（一時ファイル → rename）で対応検討

---

## 次スプリントへの持ち越し候補

| Issue | 理由 |
|-------|------|
| #36 | WebUI。設計スプリントを1本挟んでから実装を推奨 |

### 追加の技術的負債

| 課題 | 概要 |
|------|------|
| `tdsl cache clear` コマンド | キャッシュディレクトリのクリアコマンドがない |
| キャッシュファイルのアトミック書き込み | 並行実行時の競合対策 |

---

## マネージャー総評

今スプリントは計画通り #28 を完了させることができた。
全72テストが通過しており、コードベースの健全性は維持されている。

デコレータパターンによる実装は疎結合で拡張性が高く、
将来の機能追加（キャッシュ管理コマンド、TTL設定の永続化等）に備えた
良好な基盤を整えることができた。

### 次スプリントへの推奨アクション

1. **Issue #36: WebUI設計スプリント**（次スプリント）
   - 技術選定（wasm vs HTTP API、FEフレームワーク、ホスティング）から着手
   - #34・#35・#28 の完了でインフラ・レンダリング・DX が整備済み
   - WebUI 実装の前提条件がすべて揃った状態

2. **`tdsl cache clear` サブコマンド**（ユーティリティ）
   - キャッシュの管理機能を提供することでユーザビリティ向上
