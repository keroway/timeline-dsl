# 次版の公開判断資料

2026-11-15 時点の準備資料。版番号・公開可否は未確定で、今回の変更ではバージョンを上げない。
判断責任者はリポジトリ管理者 **keroway**。この資料を含む準備PRへ判断を記録してから、
別のバンプPRとタグ公開へ進む。CI成功、マージ、タグ、配布、WebUIデプロイはそれぞれ別の完了条件である。

## 比較した範囲とCHANGELOGの照合

- 前回タグ: `v2.1.0`、commit `4ab35473fd6054bd1d8c5623638144362e1603cf`（2026-09-02）。
- 確認したmain: `c14a3d821c3efe9c6ba294a13748a194343c697e`。
- `git log v2.1.0..c14a3d8 --oneline` の94コミットをCHANGELOGと照合した。
  [比較範囲](https://github.com/keroway/timeline-dsl/compare/v2.1.0...c14a3d821c3efe9c6ba294a13748a194343c697e)。
- その上に積む今回の共有URL・出力鮮度・公開状態の変更は、レビュー中のUnreleasedとして記録。
- #966のSourceSpanと#958のcoverage gateは誤って2.1.0欄に追記されていたためUnreleasedへ移した。
  WebUI履歴・非同期編集、LSP、CSV/IR往復、キーワード、依存・運用の記載不足を補った。

| 領域 | 利用者に届く変更・修正 | 照合した主なPR |
|---|---|---|
| WebUI | 履歴保存失敗の通知、保存値検証、5分未満で見送った履歴の再試行、ファイル・整形・Lintの古い応答破棄、描画設定の反映、不正Worker JSON拒否 | #985、#1002〜#1006、#1014 |
| 今回のWebUI | 共有URLの容量・時間上限と明示的復旧、最新成功プレビューだけを画像出力 | #1020、#1021 |
| CSV/IR | tagsの可逆エスケープ、空文字・Unicode空白・note/link/color・継続中期間の往復、decompileのgroup復元、range精度の保持 | #897、#917〜#919、#921、#922、#942、#967 |
| 厳格な診断 | 未宣言参照の具体的診断と非ゼロ終了、未対応calendar・重複プロパティの拒否、W212、Wikidata不正時刻の拒否・破損cacheの警告 | #873、#924、#934、#955、#964、#965、#968、#977 |
| lane生成 | 数字始まりの自動IDを修正、initの明示aliasとの衝突回避 | #949、#963 |
| LSP | lint品質警告、終了行、UTF-16補完位置、コメント/group/template/applyを含む参照・rename、aliasだけを編集 | #901、#904、#966、#969〜#971、#974、#982、#983 |
| エディタ/CLI | キーワード補完・ハイライト、zigzagの不適切な対処案を修正 | #925、#979 |
| 配布/CI | Homebrewの冪等化と補完維持、WASM export smoke、例・docs・coverage・依存更新、Ubuntu 24.04固定 | #864、#866、#867、#880、#890、#929、#930、#932、#935、#958〜#960、#1017 |
| セキュリティ関連 | Rust/npm修正版の反映。例外3件は未解消として別表に記録 | #909、#956、#1013、#1018、依存更新PR群 |
| 今回の運用 | 部分公開の状態表、タグ固定の再試行、既公開物の検証とHomebrew PR再利用 | #1022 |

依存更新の細かな版はCargo.lock / 各package-lock.jsonを正とし、まとめてCHANGELOGへ記載した。
エージェント設定・symlink・コメントのみの変更は利用者機能として列挙しない。

## 互換性の判断材料

次版をpatch/minor/majorのどれにするかはここでは決めない。特に以下を管理者が確認する。

| 境界 | 変更と対応 |
|---|---|
| CSV tags | `,` を区切りとして使う旧CSVは意図を確認して `|` へ移行。新エスケープは旧CLIが読めない場合があり、生成側と読込側を同じ版へ更新する |
| RustのSourceSpan | 公開structに `end_line: Option<u32>` が追加済み。外部の構造体リテラルは `end_line: None` 等の追加が必要。公開Rust APIのsource compatibilityを版判断へ反映する |
| JSON IR | `end_line` は省略可能で旧JSONを読める。複数行では新フィールドが出るため、独自の厳密schema利用者は更新を確認する |
| 自動lane ID | 数字始まりのラベルでは `123` → `lane_123`。IR IDを外部参照している利用者は対応表を更新する |
| 以前受理した入力 | 未対応calendar、重複プロパティ、不正な時刻claim、未宣言参照を明示エラーにする。黙って通っていた入力に依存するCIは修正が必要 |
| WebUI | 共有URLのみ容量・時間制限を追加。通常ファイル編集は制限しない。古いプレビューを画像出力する操作はできなくなる |
| 運用CLI入力 | Release workflowの手動入力は `version` から既存 `tag=vX.Y.Z` へ変更。自動呼出しがあれば修正する |

### CSV tagsの移行例

下表はCSVデコード後の **tagsセルの値**。カンマを含むセルはCSVファイルでは二重引用符で囲む。
文字置換を全CSVへ一括適用すると、本来のタグ内カンマを壊すため、列と意図を確認する。

| 意図したタグ配列 | 旧データ/注意 | 新tagsセル |
|---|---|---|
| `["alpha", "beta"]` | `alpha,beta` を区切りにしていた | `alpha|beta` |
| `["c,d"]` | 新版はカンマをタグ内文字として保持 | `c,d`（CSV上は `"c,d"`） |
| `["a|b", "c,d"]` | タグ内 `|` はエスケープする | `a\|b|c,d`（CSV上は `"a\|b|c,d"`） |
| `[""]` | 空配列 `[]` と区別 | `\e`（空配列は空セル） |
| `[" padded "]` | 境界の半角スペースを保持 | `\spadded\s` |
| `["　x　"]` | 境界の全角スペースを保持 | `\u{3000}x\u{3000}` |
| `["a\\b"]` | タグ内バックスラッシュ | `a\\b` |

最小例（2タグの意図を保つ移行）:

```csv
lane,type,start,end,time,label,tags,id
main,event,,,2020,example,alpha|beta,event-1
```

未知のエスケープや末尾の `\` はエラーになる。詳細は[CSV仕様](cli-spec.md#tags-列のエンコーディング885-954)。
移行後は `tdsl import-csv` の結果を `tdsl check` で検証し、タグ配列が意図どおりか確認する。

## 配布状況の確認記録

**2026-11-15の読み取り結果**。現時点で前回2.1.0の配布物は存在するが、今回の94コミットと
レビューPRは未公開。存在確認と新workflowが要求するSHA証跡の検証を区別する。

| 配布先 | 対象版・SHA | 確認結果・確認方法 | 未公開分の再試行 |
|---|---|---|---|
| CLI 5アーカイブ | 2.1.0 / タグ `4ab3547…` | GitHub Release APIで5件uploadedとdigestを確認。新形式のprovenanceなし、ソースSHAの同一性は未確認 | `release.yml` に既存tag。既存の証跡不足は停止して調査 |
| WASMアーカイブ | 2.1.0 / 同タグ | `tdsl-wasm-v2.1.0.tar.gz` の存在とdigestを確認。provenanceなし | 同上 |
| npm | @keroway/tdsl-wasm 2.1.0 / ソースSHA未確認 | registry HTTP 200、version=2.1.0。新形式のrelease-source証跡は今回の確認対象外 | `release.yml`。再公開せず既存tarballのintegrityとSHAを検証 |
| crates.io（4クレート） | tdsl-parser / tdsl-wikidata / tdsl-core / tdsl-render 各2.1.0 / ソースSHA未確認 | 各version API HTTP 200、num=2.1.0 | `release.yml`。checksum・cargo vcs SHAを検証し未公開crateだけpublish |
| GitHub VSIX | 2.1.0 / タグ `4ab3547…` | `timeline-dsl-2.1.0.vsix` uploadedとdigestを確認。provenanceなし | `vscode-publish.yml` に既存tag |
| Marketplace | 2.1.0 / ソースSHA未確認 | 対象版vspackageのHTTP 200を確認。VSIX内SHAは未確認 | 同上。既公開版は証跡を確認してskip |
| Homebrew | 2.1.0 / タグを参照 | tapのmain上のFormulaが2.1.0の4URLを参照し、各SHA256がGitHub asset digestと一致。PR作成だけの状態ではない。実brew installは未実施 | `release.yml`。既存PR/branchを再利用し、マージは管理者が判断 |
| WebUI Pages | deploy SHA `552607f2dc3cdfa1064d9e3f3c5822f69a5a3f72`、利用WASM 2.1.0 | [2026-11-15 deploy run](https://github.com/keroway/timeline-dsl/actions/runs/43681433873) success。そのSHAのlockfileでWASM版を確認。ライブ配信バイトの同一性は未検証 | mainのWebUI変更マージ後の `deploy-pages.yml`。タグ公開とは別に記録 |

[GitHub Release](https://github.com/keroway/timeline-dsl/releases/tag/v2.1.0)、
[tap Formula](https://github.com/keroway/homebrew-tap/blob/main/Formula/tdsl.rb)。
registryの存在確認は[Hurlファイル](release-checks.hurl)で再実行できる:

```bash
hurl --test --continue-on-error --variable version=2.1.0 docs/release-checks.hurl
```

このコマンドは存在と版だけを検査し、publishしない。SHAまで確認する次版の状態再集計手順は
[リリース手順](release.md#配布状況の状態表)を参照。今回、公開workflow・デプロイは実行していない。

## 公開前の残課題

| 項目 | 現状 / 次の対応 | 責任・期限 |
|---|---|---|
| [#1019](https://github.com/keroway/timeline-dsl/issues/1019) | JSON構文が壊れた履歴の退避はOPEN。別作業の修正・テスト・レビューを確認してからWebUI公開可否を判断。本スタックには混ぜない | 管理者、公開判断前 |
| [#1012](https://github.com/keroway/timeline-dsl/issues/1012) | 以下3例外を追跡。例外の変更・延長・解消は本PRではしない | 管理者、2027-02-08までに再確認 |
| 公開認証・配布確認 | OIDC、Marketplace token、tap tokenの実公開は未検証。fixture成功を本番公開成功と扱わない | 管理者、公開時 |
| 旧配布物の証跡 | 新しいSHA証跡が無い配布物は自動再試行で確認不能になり得る。旧版のソース対応を調査し、タグ付替え/上書きで回避しない | 管理者、旧版の再試行前 |
| 版判断 | CSV・Rust構造体・厳格化の互換性レビューを行い、次版番号を決める | 管理者、バンプPR前 |
| [#1016](https://github.com/keroway/timeline-dsl/issues/1016) | Ubuntu 26.04再検証は別課題。現行24.04固定でのCI結果を判断材料とする | 管理者、別途triage |

依存例外（リポジトリ設定と#1012の2026-11-12記録に基づく。上流の新規リリース調査は今回未実施）:

| ID | 対象・理由 | 解消条件 / 再確認期限 |
|---|---|---|
| RUSTSEC-2026-0206 | rustybuzz 0.20.1。svg2pdf経由の推移的依存、unmaintainedで修正版なしという記録 | svg2pdf/usvg系のharfrust移行を確認 / 2027-02-08 |
| RUSTSEC-2026-0192 | ttf-parser 0.25.1。fontdb/usvg/rustybuzz経由、unmaintainedという記録 | skrifa系への上流移行を確認 / 2027-02-08 |
| GHSA-gcqx-w48m-w2h4 | deepmerge 4.3.1。WebUIビルド用dev依存のprototype poisoning、修正版なしという記録。設定マージ用途で外部入力を渡さないという例外理由 | 修正版または依存除去を確認 / 2027-02-08 |

CI成功は例外込みのチェック通過を示すものであり、「脆弱性なし」を意味しない。

## 公開前チェックリスト

- [ ] 4本のレビューPRを依存順（#1020 → #1021 → #1022 → この準備PR）にレビューし、各マージ後のmain CIを確認する。
- [ ] #1019の解消状況と、WebUIがmainマージでデプロイされる境界を確認する。
- [ ] 候補SHAを固定し、v2.1.0以降の追加コミットがあればCHANGELOG・この資料を更新する。
- [ ] CSV移行・Rust/JSON利用者への影響をレビューし、次版番号を決定する。
- [ ] #1012の例外と期限、新しい依存警告を再確認する。
- [ ] バンプPRでCargo・内部依存・VSIX・lockfile・CHANGELOGを同時更新し、Rust/WebUI/E2E/配布fixtureのCI成功を確認する。
- [ ] 管理者の公開可否を準備PRに記録する。実公開は別の明示的な操作として行う。
- [ ] 公開後、14配布先の状態表を再確認し、部分失敗は該当workflowを既存タグで再試行する。
- [ ] Homebrew PRのマージとtap反映を確認する。PR作成成功だけで配布完了にしない。
- [ ] WebUIのdeploy SHAと利用WASM版を記録し、タグの配布完了と別に動作確認する。

## 隔週の公開判断

**2026-11-20（金）を初回として隔週金曜日、JSTで判断**する。
次は2026-12-04、2026-12-18。判断責任者kerowayが、公開または見送りと次の対応を準備PRへ記録する。
公開を見送っても次回判断日は維持する。期限到来だけでは公開しない。
定期PRの自動作成・スケジュール実行・自動公開は追加しない。

準備PRの判断欄（公開判断時に管理者が更新）:

| 判断日 | 候補SHA / 版 | 公開可否 | 阻害要因 | 次の対応・担当・期限 |
|---|---|---|---|---|
| 2026-11-20 | 未確定 / 未確定 | 未判断 | #1019、互換性判断、レビュー・main CI、公開認証確認 | kerowayが上記を確認し、可否と次の対応をこのPRへ記録 |

テンプレート:

```text
判断日 (JST):
判断者: keroway
候補SHA / 次版:
判断: 公開 / 見送り
根拠・検証リンク:
阻害要因（見送り時）:
次の対応 / 担当 / 期限:
次回判断日:
公開後の配布状態 / WebUI deploy SHA / WASM版:
```
