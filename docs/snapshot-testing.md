# スナップショットテスト

`examples/*.tdsl` の IR JSON および SVG 出力を [insta](https://insta.rs/) でスナップショット固定し、意図しない出力変化を CI で検出します。

## スナップショットの場所

| クレート | 対象 | ディレクトリ |
|---|---|---|
| `tdsl-core` | IR JSON | `crates/tdsl-core/src/tests/snapshots/` |
| `tdsl-render` | SVG | `crates/tdsl-render/src/snapshots/` |

## 対象ファイル

### IR JSON スナップショット（`tdsl-core`）

Wikidata 不要の静的 examples 全件を対象にしています。

- `examples/china_dynasties.tdsl`
- `examples/japanese_history.tdsl`
- `examples/world_wars.tdsl`
- `examples/sci_tech_timeline.tdsl`
- `examples/fictional_empire.tdsl`
- `examples/apollo_11.tdsl`
- `examples/internet_history.tdsl`

### SVG スナップショット（`tdsl-render`）

ファイルサイズ削減のため代表 2 件のみを対象にしています。

- `examples/china_dynasties.tdsl`
- `examples/world_wars.tdsl`

Wikidata 連携が必要なファイル（`china_with_import.tdsl`、`samurai_wikidata.tdsl` 等）はスナップショット対象外です。CI はオフライン前提のため、ネットワーク依存テストは追加しません。

## スナップショットの更新手順

意図した出力変化（新機能追加、フォーマット変更など）があった場合は以下の手順でスナップショットを更新します。

```bash
# 1. 新しいスナップショットを生成する（.snap.new ファイルが生成される）
INSTA_UPDATE=new cargo test --workspace

# 2. 差分を確認して承認する
cargo insta review

# 3. または --accept で一括承認する（差分を確認済みの場合）
cargo insta test --accept

# 4. 承認後に全テストが通ることを確認する
cargo test --workspace
```

`cargo insta review` はインタラクティブな TUI を開き、変更前後の差分を確認しながら個別に承認・拒否できます。

## CI での振る舞い

CI では `INSTA_UPDATE` を設定しないため、スナップショットが一致しない場合はテストが失敗し、差分が表示されます。

```
thread 'tests::golden::snapshot_china_dynasties_ir' panicked:
snapshot assertion for 'snapshot_china_dynasties_ir' failed
```

このとき、ローカルで上記の「更新手順」を実行してスナップショットを更新し、コミットしてください。
