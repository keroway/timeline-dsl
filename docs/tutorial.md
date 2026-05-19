# Timeline DSL チュートリアル

## Timeline DSL とは

Timeline DSL（`.tdsl`）は、年表データをテキストで宣言的に記述するためのドメイン固有言語です。C風の波括弧・セミコロン構文を採用しており、Gitによるバージョン管理や差分レビューに適しています。コンパイラは `.tdsl` ファイルをパースし、JSON IR（中間表現）に変換します。さらに Wikidata のオープンデータを QID（識別子）経由で自動取り込みでき、歴史年表を素早く構築できます。

歴史・文化・ゲーム世界観など、何らかの「時間軸を持つ構造化データ」を扱いたい方に向いています。Rust/プログラミングの知識がなくてもDSLの読み書きは可能で、テキストエディタさえあれば作業できます。Wikidata連携機能を使えば、歴史的な王朝・人物・出来事の基礎データをコマンド一発で取り込めます。

---

## インストール

### ワンラインインストール（macOS / Linux）

```sh
curl -sSfL https://raw.githubusercontent.com/keroway/timeline-dsl/main/install.sh | sh
```

インストール後は `tdsl` コマンドがパスに追加されます。

### cargo でインストール（Rust 開発者向け）

```sh
cargo install --git https://github.com/keroway/timeline-dsl tdsl-cli
```

インストールが完了したら、次のコマンドで動作確認できます。

```sh
tdsl --help
```

---

## チュートリアル A: 手作業で年表を作る

テキストを直接記述し、架空世界や独自テーマの年表を作成するフローです。Wikidata への接続は不要です。

### A-1. テンプレートを生成する（tdsl init）

`tdsl init` コマンドで、年表の骨格となる `.tdsl` ファイルを生成します。

```bash
tdsl init \
  --output my_timeline.tdsl \
  --timeline "架空世界年表" \
  --range-start 1000 \
  --range-end 1300 \
  --lanes "王国:kingdom,事件:incidents"
```

オプションの意味:

| オプション | 説明 |
|---|---|
| `--output` | 出力先のファイルパス |
| `--timeline` | 年表のタイトル |
| `--range-start` / `--range-end` | 表示範囲（整数年。負の値は紀元前） |
| `--lanes` | レーン定義。`ラベル:ID` のカンマ区切り |

生成されたファイルを確認します。

```bash
cat my_timeline.tdsl
```

### A-2. アイテムを追加する

テキストエディタで `my_timeline.tdsl` を開き、`span` / `event` / `event_range` を追記します。

```
timeline "架空世界年表" {
    title "架空世界年表";
    unit year;
    range 1000..1300;
    calendar proleptic_gregorian;
}

lane "王国" as kingdom { kind custom; order 10; }
lane "事件" as incidents { kind custom; order 20; }

// 王国の存続期間（span）
span kingdom 1001..1180 "アルカディア王国" {
    tags ["dynasty", "fictional"];
    id "span:arcadia";
};

// 点イベント（event）
event incidents 1042 "竜騎士団の創設" {
    tags ["founding", "fictional"];
    id "event:knights";
};

// 期間イベント（event_range）
event_range incidents 1175..1180 "黒霧戦争" {
    tags ["war", "fictional"];
    id "range:black_mist";
};
```

3種の時間要素の使い分け:

| 種類 | 構文 | 用途 |
|---|---|---|
| `span` | `span レーンID 開始..終了 "ラベル" {}` | 存続期間（王朝・人物の生没年など） |
| `event` | `event レーンID 年 "ラベル" {}` | 特定の時点に起きた出来事 |
| `event_range` | `event_range レーンID 開始..終了 "ラベル" {}` | 一定期間続いた出来事（戦争・災害など） |

### A-3. 品質チェックと自動修正（tdsl lint --fix）

`tdsl lint` で未定義レーン参照・重複 ID・`start > end` などの問題を検出します。`--fix` を付けると安全に自動修正されます。

```bash
# 問題の確認のみ
tdsl lint my_timeline.tdsl

# 自動修正
tdsl lint my_timeline.tdsl --fix
```

修正内容の例:

- タグの重複を除去
- 空タグを除去
- `start > end`（開始が終了より大きい）の場合は入れ替え
- `id` 未設定のアイテムに安定 ID を生成

JSON形式でCI連携する場合:

```bash
tdsl lint my_timeline.tdsl --format json
```

### A-4. HTML で可視化する（tdsl render）

`tdsl render` でスタンドアロンの HTML ファイルを生成します。ブラウザで開くだけで年表が表示されます。

```bash
tdsl render my_timeline.tdsl --output my_timeline.html
open my_timeline.html   # macOS の場合
```

スケールを大きくして読みやすくするには `--scale` オプションを使います（デフォルトは 2）。

```bash
tdsl render my_timeline.tdsl --scale 5 --output my_timeline.html
```

> HTML ファイルは外部依存なしのスタンドアロン形式です。インラインSVG + CSS のみで構成されており、JavaScript 非依存です。各要素にマウスを乗せるとラベル・期間・タグなどの詳細がツールチップで表示されます。

---

## チュートリアル B: Wikidata から年表を生成する

Wikidata の QID（エンティティ識別子）を使い、歴史的な王朝・人物・組織の年表を半自動で構築するフローです。ネットワーク接続が必要です。

### B-1. エンティティを探す（tdsl search）

年表化したい対象のキーワードで Wikidata を検索します。

```bash
tdsl search "漢王朝" --lang ja -n 5
```

出力例:

```
Q7209  漢  中国の王朝（紀元前206年〜220年）
Q8733  後漢  漢の後継王朝（25年〜220年）
...
```

`-n` は結果件数の上限です。`--lang` は表示言語の優先順位です（カンマ区切り）。

Wikipedia の URL から QID を解決することもできます。

```bash
tdsl resolve "https://ja.wikipedia.org/wiki/漢"
# -> Q7209
```

### B-2. 年表化適性を確認する（tdsl inspect）

取得した QID が年表化に必要なプロパティ（成立年・消滅年など）を持っているか確認します。

```bash
tdsl inspect Q7209 --lang ja,en
```

出力には以下の情報が含まれます。

- エンティティの基本情報（ラベル・説明）
- 年表に使えるプロパティの一覧（P571 成立年、P576 消滅年、P569 誕生年など）
- 年表化適性の診断結果

よく使うプロパティ:

| プロパティ | 意味 | DSL 式 |
|---|---|---|
| P569 | 誕生年（人物） | `claim(P569).year` |
| P570 | 死亡年（人物） | `claim(P570).year` |
| P571 | 成立年（組織・王朝） | `claim(P571).year` |
| P576 | 消滅年（組織・王朝） | `claim(P576).year` |
| P580 | 開始時点 | `claim(P580).year` |
| P582 | 終了時点 | `claim(P582).year` |

### B-3. .tdsl 雛形を生成する（tdsl scaffold wikidata）

QID のリストから `.tdsl` の雛形を自動生成します。

```bash
tdsl scaffold wikidata \
  --qids Q7183,Q7209 \
  --timeline "中国王朝(生成)" \
  --lang ja,en \
  --target auto \
  --lane-mode per-entity \
  --output china_scaffold.tdsl
```

オプションの意味:

| オプション | 説明 |
|---|---|
| `--qids` | 対象エンティティの QID（カンマ区切り） |
| `--timeline` | 年表のタイトル |
| `--lang` | ラベルの言語優先順位 |
| `--target auto` | `span` / `event` を自動判定 |
| `--lane-mode per-entity` | エンティティごとにレーンを生成 |
| `--output` | 出力先ファイルパス |

生成された `.tdsl` ファイルには `import` ブロックと `map` ブロックが含まれます。

```
import wikidata as wd {
    entity Q7183 as qin_dynasty;
    entity Q7209 as han_dynasty;
    policy merge_by_source;
}

map wd.qin_dynasty to span {
    lane qin;
    start claim(P571).year;
    end claim(P576).year;
    label label@ja ?? label@en;
    tags ["dynasty", "imported"];
}
```

### B-4. 構文チェック（tdsl check）

生成された雛形に問題がないか確認します。

```bash
tdsl check china_scaffold.tdsl
```

エラーがなければ次のステップへ進みます。エラーが表示された場合は、エラーメッセージに示されている行番号を参考にファイルを修正してください。

> `tdsl check` は静的なパース・意味検証のみを行い、Wikidata にはアクセスしません。Wikidata データの取得は `tdsl build` / `tdsl render` 実行時に行われます（`--offline` を指定した場合はスキップ）。

### B-5. HTML で可視化する（tdsl render）

```bash
tdsl render china_scaffold.tdsl --output china_scaffold.html
open china_scaffold.html   # macOS の場合
```

Wikidata にアクセスできない環境では `--offline` を付けます。

```bash
tdsl render china_scaffold.tdsl --offline --output china_scaffold.html
```

---

## よくある質問（FAQ）

### 1. 紀元前の年はどう書く？

負の整数で表現します。例えば紀元前 206 年は `-206`、紀元前 221 年は `-221` です。

```
span qin -221..-206 "秦" { tags ["dynasty"]; };
```

`range` の開始にも負の整数が使えます。

```
timeline "古代年表" {
    range -500..500;
}
```

### 2. Wikidata にない事象はどう追加する？

`event` / `event_range` / `span` を直接 `.tdsl` ファイルに記述します。

```
event han -209 "陳勝・呉広の乱" {
    tags ["revolt"];
    id "event:chen_sheng";
};
```

Wikidata からインポートしたファイルに追記する場合も、同じ文法でアイテムを追加できます。

スプレッドシートで管理しているアイテムを取り込みたい場合は、`tdsl import-csv` を使うと CSV を `.tdsl` スニペットに変換できます。`start` / `end` / `time` 列は `YYYY-MM-DD` / `YYYY-MM` / `YYYY` の 3 精度に対応しています（紀元前は年精度のみ）。

```bash
# CSV を標準出力に変換
tdsl import-csv examples/fictional_empire_items.csv

# 既存ファイルに追記
tdsl import-csv items.csv --append my_timeline.tdsl
```

詳細は [docs/cli-spec.md#import-csv](cli-spec.md#import-csv) を参照してください。

### 3. エラーメッセージが出た時はどうする？

エラーメッセージには行番号と原因が表示されます。主なパターン:

- **未定義レーン参照**: `span` / `event` で使ったレーン ID が `lane` 宣言に存在しない。レーン ID のスペルを確認してください。
- **start > end**: 開始年が終了年より大きい。`tdsl lint --fix` で自動修正できます。
- **Wikidata fetch 失敗**: ネットワーク接続またはレート制限の問題。`tdsl check` は Wikidata にアクセスしないため構文チェックは常に可能です。Wikidata 取得が必要な `tdsl build` / `tdsl render` では `--offline` を付けることでスキップできます。
- **パースエラー**: セミコロンや波括弧が抜けている可能性があります。エラー行周辺の構文を確認してください。

```bash
# エラーの詳細を確認する
tdsl check my_timeline.tdsl

# 自動修正できる問題は lint --fix で対処
tdsl lint my_timeline.tdsl --fix
```

### 4. lane（レーン）を複数定義するには？

`lane` 宣言を複数並べるだけです。`order` でレーンの表示順を制御できます（数値が小さいほど上に表示）。

```
lane "秦" as qin     { kind dynasty; order 10; }
lane "漢" as han     { kind dynasty; order 20; }
lane "三国" as sanguo { kind dynasty; order 30; }
```

`kind` は分類ラベルで、`dynasty`（王朝）・`person`（人物）・`nation`（国）などを自由に指定できます。`as` で内部 ID を明示しない場合、ラベルから ASCII スラッグが自動生成されます（日本語ラベルのみの場合は `lane_1`、`lane_2`... のように自動採番されます）。

### 5. Wikidata API にアクセスできない環境での使い方は？

`tdsl build` / `tdsl render` に `--offline` フラグを付けると Wikidata へのリクエストをスキップします。

```bash
# オフラインでビルド（Wikidata fetch をスキップ）
tdsl build my_timeline.tdsl --offline --pretty

# オフラインでレンダリング
tdsl render my_timeline.tdsl --offline --output out.html
```

> `tdsl check` は常に静的検証のみを実行し Wikidata にはアクセスしないため、`--offline` は不要です。

オフラインモードでは `import` / `map` ブロックによるデータ取り込みが行われないため、Wikidata 由来のアイテムは出力に含まれません。静的に定義したアイテム（`span` / `event` / `event_range`）は通常通り処理されます。

### 6. キャッシュをリセットするには？

Wikidata 取得キャッシュは `~/.cache/tdsl/` に保存されています。`tdsl cache` コマンドで管理できます。

```bash
# キャッシュの状態を確認
tdsl cache status

# すべてのキャッシュを削除
tdsl cache clear

# 30日以上古いキャッシュを削除
tdsl cache clear --older-than 30
```

---

## 次のステップ

- **DSL 仕様の詳細**: [docs/dsl-spec.md](dsl-spec.md) — 文法リファレンス・全プロパティ・IR構造の完全な仕様
- **サンプルファイル**:
  - `examples/china_dynasties.tdsl` — 静的定義のみのシンプルなサンプル
  - `examples/china_with_import.tdsl` — Wikidata 連携のサンプル
  - `examples/fictional_empire.tdsl` — 架空世界向けサンプル
