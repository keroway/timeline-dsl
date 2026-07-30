# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.x     | Yes       |
| 0.x     | No        |

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Use GitHub's [Private vulnerability reporting](https://github.com/keroway/timeline-dsl/security/advisories/new) to report security issues privately. We will respond within 7 days.

### What to include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (optional)

## Dismissed advisories（許容済みアラート）

到達不能・実害なしと判断して Dismiss した Dependabot アラートを記録する。再評価条件に該当する変化があれば見直すこと。

### #3 — uuid: Missing buffer bounds check in v3/v5/v6 when buf is provided

- **GHSA**: `GHSA-w5hq-g745-h8pq`（対象: `apps/webui/package-lock.json`）
- **経路**: `uuid@10.0.0` ← `vite-plugin-top-level-await@1.6.0`（`apps/webui` の唯一の利用元）
- **Dismiss 理由**（`not_used` = Vulnerable code is not actually used）:
  - 脆弱性は v3/v5/v6 に **`buf` 引数を渡したとき**のバッファ境界チェック欠落。利用元 `vite-plugin-top-level-await` は `v5(seed, namespace)` を **buf 引数なし**で呼ぶため、脆弱なコードパスに到達しない。
  - `vite-plugin-top-level-await` は**ビルド時専用**ツールであり、uuid はブラウザバンドルに同梱されない。
- **修正版に追従しない理由（trade-off）**:
  - `vite-plugin-top-level-await` は 1.6.0 が最新で `uuid: "10.0.0"` を**完全固定**しているため、`overrides` を使わない限り修正版（uuid 14）は入らない。
  - `uuid@14` は **ESM 専用**（`exports` に `require` 条件なし）。`overrides` で強制した場合、プラグインの `require("uuid")` は Node の require(ESM) サポート（Node 20.19+ / 22.12+ / 24 / 26）に依存する。技術的には動作し得る（ローカルは Node v26）が、Node バージョン依存の脆さを増やす。上記のとおり到達不能・実害なしのため、override より dismiss を選択した。
- **再評価条件**:
  - より厳密に消したい場合は `apps/webui/package.json` に `"overrides": { "uuid": "14.0.0" }` を入れ、`npm install` 後に `npm run build` が通ることを確認して採用してもよい（require(ESM) 依存に留意）。
  - `vite-plugin-top-level-await` が uuid の参照を更新（CJS 互換の修正版に対応／range 化）したら、追従または `overrides` での `uuid` バンプを検討する。
  - プラグインが `buf` 引数を渡す実装に変わった場合は到達性を再判定する。
