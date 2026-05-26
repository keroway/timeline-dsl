//! `textDocument/codeAction` レスポンスの生成。
//!
//! fixable な lint issue が存在するとき「Fix all fixable tdsl lint issues」
//! アクションを返す。適用するとドキュメント全体を `apply_lint_fixes` +
//! `render_tdsl_file` で書き直す。

use std::collections::HashMap;

use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Position, Range, TextEdit,
    Url, WorkspaceEdit,
};

/// DSL ソーステキストと URI から Code Action のリストを生成する。
///
/// - パースが失敗した場合は空 vec を返す。
/// - fixable な lint issue が 1 件もなければ空 vec を返す。
/// - fix 後のテキストが元と同一なら空 vec を返す。
pub fn compute_code_actions(
    uri: &Url,
    source: &str,
    _params: &CodeActionParams,
) -> Vec<CodeActionOrCommand> {
    let Ok(mut file) = tdsl_parser::parse(source) else {
        return vec![];
    };

    let issues = tdsl_core::lint::lint_issues(&file, source);
    if !issues.iter().any(|i| i.fixable) {
        return vec![];
    }

    tdsl_core::lint::apply_lint_fixes(&mut file);
    let fixed_text = tdsl_core::lint::render_tdsl_file(&file);
    if fixed_text == source {
        return vec![];
    }

    // ドキュメント全体を置換する TextEdit。
    // 末尾行と末尾列を正確に算出する。
    let line_count = source.lines().count() as u32;
    let last_line_len = source.lines().last().map(|l| l.len()).unwrap_or(0) as u32;
    let whole_doc = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: line_count,
            character: last_line_len,
        },
    };

    let edit = TextEdit {
        range: whole_doc,
        new_text: fixed_text,
    };

    let action = CodeAction {
        title: "Fix all fixable tdsl lint issues".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
            document_changes: None,
            ..Default::default()
        }),
        ..Default::default()
    };

    vec![CodeActionOrCommand::CodeAction(action)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::*;

    fn dummy_params(uri: &Url) -> CodeActionParams {
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            context: CodeActionContext {
                diagnostics: vec![],
                only: None,
                trigger_kind: None,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
        }
    }

    fn test_uri() -> Url {
        Url::parse("file:///test.tdsl").unwrap()
    }

    /// fixable な issue がなければ code action は返らない。
    #[test]
    fn no_fixable_issues_returns_empty() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 10..20 "S" { id "s1"; };
"#;
        let uri = test_uri();
        let params = dummy_params(&uri);
        let actions = compute_code_actions(&uri, src, &params);
        assert!(
            actions.is_empty(),
            "正常な DSL は code action を返すべきでない。実際: {actions:#?}"
        );
    }

    /// start_gt_end を持つ DSL → code action 1 件が返る。
    #[test]
    fn fixable_issue_returns_action() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 50..10 "S" { id "s1"; };
"#;
        let uri = test_uri();
        let params = dummy_params(&uri);
        let actions = compute_code_actions(&uri, src, &params);
        assert_eq!(
            actions.len(),
            1,
            "start_gt_end があれば code action 1 件を返すべき。実際: {actions:#?}"
        );

        // action の中身を検証
        if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
            assert_eq!(action.title, "Fix all fixable tdsl lint issues");
            assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
            let edit = action.edit.as_ref().unwrap();
            let changes = edit.changes.as_ref().unwrap();
            assert!(changes.contains_key(&uri));
        } else {
            panic!("expected CodeAction, got command");
        }
    }

    /// code action の new_text を適用した結果が `apply_lint_fixes + render_tdsl_file` と一致する。
    #[test]
    fn fix_action_produces_same_result_as_lint_fix() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 50..10 "S" {};
"#;
        let uri = test_uri();
        let params = dummy_params(&uri);
        let actions = compute_code_actions(&uri, src, &params);
        assert!(!actions.is_empty(), "fixable issue は action を返すべき");

        // action が提示する new_text を取得
        let new_text = if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
            let changes = action.edit.as_ref().unwrap().changes.as_ref().unwrap();
            changes[&uri][0].new_text.clone()
        } else {
            panic!("expected CodeAction");
        };

        // apply_lint_fixes + render_tdsl_file の結果と一致すること
        let mut file = tdsl_parser::parse(src).unwrap();
        tdsl_core::lint::apply_lint_fixes(&mut file);
        let expected = tdsl_core::lint::render_tdsl_file(&file);

        assert_eq!(
            new_text, expected,
            "code action の new_text は apply_lint_fixes + render_tdsl_file と一致すべき"
        );
    }

    /// fix を適用した後、fixable な lint issue がなくなること。
    #[test]
    fn fix_action_resolves_fixable_issues() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 50..10 "S" {};
event a 30 "E" {};
event_range a 80..20 "R" { tags ["x", "x"]; };
"#;
        let uri = test_uri();
        let params = dummy_params(&uri);
        let actions = compute_code_actions(&uri, src, &params);
        assert!(!actions.is_empty());

        let new_text = if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
            let changes = action.edit.as_ref().unwrap().changes.as_ref().unwrap();
            changes[&uri][0].new_text.clone()
        } else {
            panic!("expected CodeAction");
        };

        // 修正後のテキストを再パースして lint issues を確認
        let reparsed = tdsl_parser::parse(&new_text).unwrap();
        let remaining = tdsl_core::lint::lint_issues(&reparsed, &new_text);
        let fixable_remaining: Vec<_> = remaining.iter().filter(|i| i.fixable).collect();
        assert!(
            fixable_remaining.is_empty(),
            "fix 後に fixable な issue が残るべきでない。残っている: {fixable_remaining:#?}"
        );
    }

    /// パース不能な DSL → code action は返らない。
    #[test]
    fn parse_error_returns_empty() {
        let src = r#"timeline "broken" { title"#;
        let uri = test_uri();
        let params = dummy_params(&uri);
        let actions = compute_code_actions(&uri, src, &params);
        assert!(
            actions.is_empty(),
            "パース不能な DSL は code action を返すべきでない"
        );
    }
}
