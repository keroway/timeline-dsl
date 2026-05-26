//! `textDocument/codeAction` の純粋ロジック。
//!
//! `tdsl lint --fix` 相当の安全な自動修正を quick fix として提供する。
//! MVP では「自動修正可能な lint をすべて修正」する**単一の全文置換**アクションを返す
//! （`tdsl lint --fix` と同じ振る舞い = 全文再 emit のため、コメントは整形時に失われる）。
//!
//! - fixable な lint issue が 1 件も無い場合はアクションを返さない（空 vec）。
//! - fixable でない issue（`unknown_lane` / `duplicate_id` / `empty_label`）しか無い場合も空 vec。
//! - ネットワーク I/O は行わない（offline 前提・CI 安全）。

use std::collections::HashMap;

use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, DocumentChanges, OneOf,
    OptionalVersionedTextDocumentIdentifier, Position, Range, TextDocumentEdit, TextEdit, Url,
    WorkspaceEdit,
};

use crate::hover::byte_offset_to_utf16;

/// Code Action 要求を処理して LSP CodeAction のリストを返す。
///
/// カーソル位置の `range` は現状のロジックでは未使用（全文を対象に lint する）。
/// fixable な lint issue があり、かつ修正で内容が変化する場合のみ quick fix を 1 件返す。
///
/// `version` は要求時点のドキュメントバージョン。`supports_document_changes` が `true`
/// （client が `workspace.workspaceEdit.documentChanges` をサポート）の場合、全文置換は
/// **バージョン付きの `documentChanges`** として返すため、コードアクション計算後に
/// ドキュメントが変更されると client 側がバージョン不一致を検出して stale な全文置換の
/// 適用を拒否する（ユーザーの新しい編集を上書きしない）。非対応クライアントには
/// `changes`（バージョン保護なし）にフォールバックする。
pub fn compute_code_actions(
    source: &str,
    uri: &Url,
    version: i32,
    supports_document_changes: bool,
    _range: Range,
) -> Vec<CodeActionOrCommand> {
    // パースできなければ Code Action を出さない（診断側でエラー表示される）
    let Ok(file) = tdsl_parser::parse(source) else {
        return Vec::new();
    };

    // fixable な lint issue の件数を数える
    let fixable_count = tdsl_core::lint::lint_issues(&file, source)
        .iter()
        .filter(|issue| issue.fixable)
        .count();
    if fixable_count == 0 {
        return Vec::new();
    }

    // 実際に修正を適用した結果を取得（変更が無ければ None）。
    // fixable issue があっても apply_lint_fixes 後に内容が変わらないケースは
    // 理屈上ないが、None なら安全側に倒してアクションを出さない。
    let fixed = match tdsl_core::lint::fix_source(source) {
        Ok(Some(fixed)) => fixed,
        Ok(None) | Err(_) => return Vec::new(),
    };

    // 全文を置換する TextEdit を構築する
    let edit = TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: document_end_position(source),
        },
        new_text: fixed,
    };

    let workspace_edit = if supports_document_changes {
        // バージョン付き documentChanges。要求時点のバージョンを載せることで、計算後に
        // 編集されたドキュメントへの stale な全文置換適用を client が拒否する。
        let document_edit = TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: Some(version),
            },
            edits: vec![OneOf::Left(edit)],
        };
        WorkspaceEdit {
            document_changes: Some(DocumentChanges::Edits(vec![document_edit])),
            ..Default::default()
        }
    } else {
        // documentChanges 非対応クライアントへのフォールバック。
        // バージョン保護はできないが、`changes` でないと適用されないクライアントのために
        // 互換経路を残す。
        let mut changes = HashMap::new();
        changes.insert(uri.clone(), vec![edit]);
        WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }
    };

    let action = CodeAction {
        title: format!("tdsl: 自動修正可能な lint をすべて修正 ({fixable_count} 件)"),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(workspace_edit),
        ..Default::default()
    };

    vec![CodeActionOrCommand::CodeAction(action)]
}

/// ドキュメント末尾の LSP `Position`（0-based・UTF-16 列）を返す。
///
/// 末尾の改行も考慮するため `split('\n')` で分割する（`lines()` は末尾の空行を落とすため使わない）。
fn document_end_position(source: &str) -> Position {
    let mut line: u32 = 0;
    let mut last_line = "";
    for (i, l) in source.split('\n').enumerate() {
        line = i as u32;
        last_line = l;
    }
    Position {
        line,
        character: byte_offset_to_utf16(last_line, last_line.len()) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri() -> Url {
        Url::parse("file:///test.tdsl").unwrap()
    }

    fn whole_range() -> Range {
        Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        }
    }

    fn extract_action(actions: &[CodeActionOrCommand]) -> &CodeAction {
        match &actions[0] {
            CodeActionOrCommand::CodeAction(a) => a,
            CodeActionOrCommand::Command(_) => panic!("expected CodeAction, got Command"),
        }
    }

    #[test]
    fn offers_quick_fix_for_fixable_issues() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 50..10 "S" { tags ["x", "", "x"]; };
event a 30 "E" {};
"#;
        let actions = compute_code_actions(src, &uri(), 7, true, whole_range());
        assert_eq!(actions.len(), 1, "expected one quick fix");

        let action = extract_action(&actions);
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        assert!(
            action.title.contains("件"),
            "title should include fixable count: {}",
            action.title
        );

        // バージョン付き documentChanges として返り、要求時バージョンが載ること
        let edit = action.edit.as_ref().expect("workspace edit present");
        let doc_changes = edit
            .document_changes
            .as_ref()
            .expect("document_changes present");
        let new_text = match doc_changes {
            DocumentChanges::Edits(edits) => {
                assert_eq!(edits.len(), 1, "single document edit");
                let tde = &edits[0];
                assert_eq!(
                    tde.text_document.version,
                    Some(7),
                    "edit must carry the requested document version"
                );
                assert_eq!(tde.edits.len(), 1, "single whole-document edit");
                match &tde.edits[0] {
                    OneOf::Left(te) => te.new_text.clone(),
                    OneOf::Right(_) => panic!("expected plain TextEdit"),
                }
            }
            DocumentChanges::Operations(_) => panic!("expected DocumentChanges::Edits"),
        };

        // 置換テキストを再パースすると fixable issue が消えていること
        let reparsed = tdsl_parser::parse(&new_text).unwrap();
        let issues = tdsl_core::lint::lint_issues(&reparsed, &new_text);
        assert!(
            !issues.iter().any(|i| matches!(
                i.code.as_str(),
                "start_gt_end" | "invalid_tags" | "missing_id"
            )),
            "fixable issues should be gone after applying fix, got: {issues:?}"
        );
    }

    #[test]
    fn falls_back_to_changes_when_document_changes_unsupported() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
event a 30 "E" {};
"#;
        let actions = compute_code_actions(src, &uri(), 3, false, whole_range());
        assert_eq!(actions.len(), 1, "expected one quick fix");

        let action = extract_action(&actions);
        let edit = action.edit.as_ref().expect("workspace edit present");
        // 非対応クライアントには changes（バージョン無し）で返す
        assert!(
            edit.document_changes.is_none(),
            "must not emit documentChanges when unsupported"
        );
        let changes = edit.changes.as_ref().expect("changes present as fallback");
        let edits = changes.get(&uri()).expect("edits for uri");
        assert_eq!(edits.len(), 1, "single whole-document edit");
        assert!(
            tdsl_parser::parse(&edits[0].new_text).is_ok(),
            "fallback edit text must be valid tdsl"
        );
    }

    #[test]
    fn no_action_for_clean_source() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 10..20 "S" { tags ["x", "y"]; id "s1"; };
"#;
        let actions = compute_code_actions(src, &uri(), 1, true, whole_range());
        assert!(actions.is_empty(), "clean source should offer no actions");
    }

    #[test]
    fn no_action_for_only_non_fixable_issues() {
        // unknown_lane / duplicate_id / empty_label はいずれも fixable=false
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
event ghost 10 "E1" { id "dup"; };
event a 20 "E2" { id "dup"; };
event a 30 "" { id "e3"; };
"#;
        let actions = compute_code_actions(src, &uri(), 1, true, whole_range());
        assert!(
            actions.is_empty(),
            "non-fixable-only issues should offer no quick fix, got {} actions",
            actions.len()
        );
    }

    #[test]
    fn no_action_for_unparseable_source() {
        let actions = compute_code_actions("not valid {{{", &uri(), 1, true, whole_range());
        assert!(
            actions.is_empty(),
            "unparseable source should offer no actions"
        );
    }

    #[test]
    fn document_end_position_handles_trailing_newline() {
        assert_eq!(
            document_end_position("a\n"),
            Position {
                line: 1,
                character: 0
            }
        );
        assert_eq!(
            document_end_position("a\nbc"),
            Position {
                line: 1,
                character: 2
            }
        );
    }
}
