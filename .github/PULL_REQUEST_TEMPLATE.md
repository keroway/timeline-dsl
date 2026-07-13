<!--
Conventional Commits（feat: / fix: / docs: / chore: / refactor: / test: / perf:）に従ってください。
1 PR 1 目的。複数の独立変更を 1 PR に混ぜないでください。
-->

## 目的

<!-- この PR で何を達成するか。関連 Issue があれば `Closes #123` で紐付け -->

## 変更点

<!-- 主要な変更を箇条書きで -->

-

## 検証手順

<!-- レビュアーが動作確認できる手順。該当するものにチェック -->

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash scripts/e2e-smoke.sh
# WebUI を触っていれば
# ( cd apps/webui && npm run lint && npm run build )
```

- [ ] 上記のローカルゲートを通過した
- [ ] 文法変更を含む場合、`keywords.json` / `dsl-spec.md` を更新した
- [ ] 仕様変更を含む場合、docs（README / CHANGELOG など）を更新した
- [ ] 既存の `examples/` がそのまま通ることを確認した

## スコープ外

<!-- この PR で「あえてやらないこと」。次以降に回す範囲を明記 -->
