# Migration Guide: v0.1.0 → v1.0.0

v0.1.0 から v1.0.0 への移行ガイドです。

## Breaking Changes

**Breaking Change はありません。** v0.1.0 で作成した `.tdsl` ファイルはそのまま v1.0.0 でも動作します。

ただし、以下の **廃止済み機能**（v0.1.0 時点で既に廃止）は引き続き使用できません：

### `map` ブロック内の `source` プロパティ（廃止済み）

v0.1.0 以降、`map` ブロック内での `source` の手動指定は不要です。
imported item の `source` は `wd:<entity_id>` として自動付与されます。

```tdsl
// NG（v0.1.0 以前の書き方）
map wd.han_dynasty to span {
    lane han;
    source wd:Q7209;  // <-- 廃止済み、パースエラーになります
}

// OK
map wd.han_dynasty to span {
    lane han;
    // source は自動付与されます
}
```

## 新機能

### template / apply 構文

共通フォーマットを再利用できるようになりました。

```tdsl
template dynasty_span {
    target_type span;
    start claim(P571).year;
    end claim(P576).year;
    tags ["dynasty"];
}

apply dynasty_span to wd.han_dynasty {
    lane han;
    label label@ja ?? label@en;
}
```

### Wikidataキャッシュ

Wikidata から取得したデータが `~/.cache/tdsl/` にキャッシュされます。
デフォルトの TTL は 24 時間です。

```bash
# キャッシュを使わず強制取得
tdsl build file.tdsl --no-cache

# TTL を1時間に設定
tdsl build file.tdsl --cache-ttl 3600

# オフラインモード（キャッシュのみ使用）
tdsl build file.tdsl --offline
```

### SVG直接出力

`tdsl render` でスタンドアロン SVG ファイルを出力できます。

```bash
tdsl render file.tdsl --format svg --output timeline.svg
```

### VS Code 構文ハイライト

`editors/vscode/` を VS Code の拡張フォルダにコピーするだけで構文ハイライトが有効になります。

```bash
cp -r editors/vscode ~/.vscode/extensions/timeline-dsl
# VS Code を再起動
```

### Homebrew インストール

```bash
brew tap keroway/tap
brew install tdsl
```

## インストール方法の更新

v1.0.0 から Homebrew によるインストールが利用可能になりました。

| 方法 | コマンド |
|---|---|
| Homebrew（推奨） | `brew tap keroway/tap && brew install tdsl` |
| ワンライン | `curl -sSfL .../install.sh \| sh` |
| cargo | `cargo install --git https://github.com/keroway/timeline-dsl tdsl-cli` |
