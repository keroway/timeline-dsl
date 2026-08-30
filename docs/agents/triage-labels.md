# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `needs-human`        | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

右列はワークスペース共通の正典（`agent-assets` の `docs/agents/triage-labels.md`）に従う。
このリポジトリ単独で書き換えないこと。

## `refined` は `ready-for-agent` へリネーム済み（2026-08-30）

このリポジトリには `refined`（「スコープ確定・受け入れ条件定義済み（細分化完了）」）が
先に存在し、canonical な `ready-for-agent` と同義だった。同義ラベルを2つ並立させると、
人にもエージェントにもどちらを付けるべきか曖昧になるため、**新規作成ではなく
`gh label edit --new-name` でリネームした**（付与済み28件はそのまま追随。`refined` を
参照するドキュメント・ワークフローは無かったため、文言の追従修正も不要だった）。

古い issue やコメントに `refined` という語が残っている場合は `ready-for-agent` と
読み替えること。

## Why `ready-for-human` maps to `needs-human`

`needs-human`（「人間の判断・操作待ち(無人ループ対象外)」）は triage スキルより先に
存在し、`ready-for-human` と同義。同義ラベルを2つ並立させると、無人ループの除外リストに
片方だけが載る事故が起きる。

**この写像は agent-assets が持つワークスペース共通の正典であり、timeline-dsl 固有の
選択ではない。** 除外ラベルの全一覧、negative gate（除外ラベルが無ければ着手可）と
positive gate（`ready-for-agent` のみ着手可）の関係、SKILL 層とスクリプト層
（`repo-probe.sh` の `eligible_issues` / `scheduled-issue-sprint.sh`）の同期義務は
`agent-assets` の `docs/agents/triage-labels.md` を正とする。ここに再掲すると必ず乖離するので、
このファイルは写像表だけを持つ。

`area:*` / `dependencies` はこのリポジトリ固有の分類ラベルで、triage ロールとは
直交する（[`issue-tracker.md`](issue-tracker.md) 参照）。
