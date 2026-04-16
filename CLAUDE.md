# CLAUDE.md -- Timeline DSL プロジェクト指示書

## プロジェクト概要

年表特化のDSLコンパイラ。`.tdsl` ファイルをパースし、WikidataからデータをインポートしてJSON IRに変換する。Rustで実装。

## ビルド・テスト

```bash
# ビルド
cargo build --workspace

# 全テスト実行
cargo test --workspace

# 特定クレートのテスト
cargo test -p tdsl-parser
cargo test -p tdsl-core
cargo test -p tdsl-wikidata

# CLIの実行（例）
cargo run -p tdsl-cli -- build examples/china_dynasties.tdsl --pretty
cargo run -p tdsl-cli -- check examples/china_dynasties.tdsl
cargo run -p tdsl-cli -- build examples/china_with_import.tdsl --pretty
cargo run -p tdsl-cli -- build examples/china_with_import.tdsl --offline --pretty
cargo run -p tdsl-cli -- fetch Q7209 --lang ja,en
```

## アーキテクチャ

### クレート構成（依存方向: cli → core → parser, core → wikidata）

```
crates/
├── tdsl-parser/    # PEG文法(pest) → AST
│   ├── grammar.pest   # PEG文法定義
│   ├── ast.rs         # AST型定義
│   ├── builder.rs     # pest解析木 → AST変換
│   └── error.rs       # パースエラー
├── tdsl-core/      # AST → IR変換・バリデーション
│   ├── ir.rs          # IR型定義（JSON直列化対象）
│   ├── lower.rs       # 4パスlowering（静的 / Wikidata連携）
│   ├── validate.rs    # 意味検証
│   └── error.rs       # lowering エラー
├── tdsl-wikidata/  # Wikidata APIクライアント
│   ├── client.rs      # WikidataClient trait + HTTP実装
│   ├── entity.rs      # エンティティ型 + 時間パース
│   └── error.rs       # Wikidataエラー
└── tdsl-cli/       # CLIバイナリ
    └── main.rs        # build / check / ast / fetch サブコマンド
```

### コンパイルパイプライン

1. **パース**: `.tdsl` → `Vec<Statement>`（AST）
2. **Lowering Pass 1**: timeline/lane 宣言を収集
3. **Lowering Pass 2**: 静的アイテム（span/event/event_range）を変換
4. **Lowering Pass 3**: import ブロックを解決（Wikidata fetch）
5. **Lowering Pass 4**: map ブロックを適用してアイテム生成
6. **バリデーション**: range整合性、未使用lane等の警告

### IR構造（`tdsl_core::ir::TimelineIr`）

- `meta`: title, unit, range, calendar
- `lanes`: id, label, kind, order
- `items`: Span / Event / EventRange（tagged enum）
- `imports`: インポート記録
- `sources`: 出典・ライセンス情報

## コーディング規約

- **Edition**: Rust 2024
- **エラー処理**: `thiserror` でエラー型定義、`miette` で整形出力
- **非同期**: `tokio`（Wikidata fetch に使用）
- **パーサ**: `pest` PEGパーサジェネレータ。文法変更は `grammar.pest` を編集後 `builder.rs` も更新すること
- **テスト**: 各クレートの `lib.rs` 末尾に `#[cfg(test)]` で統合テスト
- **シリアライズ**: IR型は `serde::Serialize + Deserialize` を derive

## DSL文法の変更手順

1. `crates/tdsl-parser/src/grammar.pest` を編集
2. `crates/tdsl-parser/src/ast.rs` にAST型を追加/変更
3. `crates/tdsl-parser/src/builder.rs` に変換ロジックを実装
4. `crates/tdsl-core/src/lower.rs` にloweringロジックを追加
5. 必要に応じて `crates/tdsl-core/src/ir.rs` のIR型を更新
6. `cargo test --workspace` で全テスト通過を確認

## Wikidataプロパティ（頻用）

| プロパティ | 意味 | DSL式 |
|---|---|---|
| P569 | 誕生年 | `claim(P569).year` |
| P570 | 死亡年 | `claim(P570).year` |
| P571 | 成立年 | `claim(P571).year` |
| P576 | 消滅年 | `claim(P576).year` |
| P580 | 開始時点 | `claim(P580).year` |
| P582 | 終了時点 | `claim(P582).year` |

## 現在のMVP実装状況

### 実装済み

- PEG文法 + パーサ（7種のstatement: timeline, lane, span, event, event_range, import, map）
- AST → IR変換（静的 / Wikidata連携 両方）
- Wikidata HTTPクライアント（wbgetentities API）
- CLI 4サブコマンド（build / check / ast / fetch）
- JSON IR出力（`origin` フィールドを含む）
- コメント（行 `//` / ブロック `/* */`）
- `map` の `target_type` は enum 型（span / event / event_range のみ許可）
- imported item の `source` は `wd:<entity_id>` で自動付与（map 内での手動指定は廃止）
- 日本語 lane 名で `as` 省略時、ASCII slug が空なら `lane_N` を自動採番
- 静的アイテム（event / event_range）の `source` も `sources[]` に登録

### 未実装（今後の拡張）

- `template` / `apply` 構文（パーサ未実装）
- 再インポートポリシーの完全な実装（パース済みだが lowering では未使用）
- `query "SPARQL" as alias`（MVPから除外済み）
- CSV/スプレッドシート変換
- Web UIエディタ
- レンダリング/可視化エンジン

## サンプルファイル

- `examples/china_dynasties.tdsl` -- 静的定義のみ（インポートなし）
- `examples/china_with_import.tdsl` -- Wikidata連携つき

## 注意点

- Wikidata APIにはレート制限あり。大量fetchする場合は `--offline` で開発し、最終確認時にオンラインビルド
- 負の年（紀元前）は整数で表現: `-206` = 紀元前206年
- lane IDの `as` 省略時はラベルからASCIIスラッグを自動生成。日本語のみの場合は `lane_N` に自動採番
- `source wd:QXXX` はWikidata出典を表し、IR の sources に CC0 ライセンスとして記録
- map ブロックの `source` プロパティは廃止済み。imported item の source は `wd:<entity_id>` で自動付与
- map の `target_type` は `span` / `event` / `event_range` のみ。不正値はパースエラー
- `wd.xxx` の entity_key が import に存在しない場合はエラー（全件フォールバックしない）
