//! `textDocument/rename` と `textDocument/prepareRename` の純粋ロジック。
//!
//! ## MVP スコープ
//! - 明示的に `as <alias>` を持つ lane ID のみリネーム対象とする。
//! - `as` 省略 lane（ラベル由来スラッグ / `lane_N` 自動採番）は prepareRename で None を返して拒否する。
//!
//! ## slug 妥当性規則
//! 新名称は `^[A-Za-z0-9_]+$` かつ空でないこと。
//!
//! ## 衝突チェック
//! 新名称が既存の他の lane ID と一致する場合はエラーを返す。
//!
//! ネットワーク I/O は行わない（offline 前提・CI 安全）。

use std::collections::HashMap;

use tower_lsp::lsp_types::{Position, Range, TextEdit, Url, WorkspaceEdit};

use crate::find_references::{
    build_line_offsets, byte_offset_to_position, compute_references, find_keyword_token_range,
};
use crate::hover::word_at_position;

// ---------------------------------------------------------------------------
// 内部ヘルパー
// ---------------------------------------------------------------------------

/// slug として有効か検証する（`^[A-Za-z0-9_]+$`、空でない）。
fn is_valid_slug(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// ソースをパースして IR を取得し、カーソル下のトークンが
/// `as <alias>` 明示の lane ID であるかを確認する。
///
/// 戻り値: `Some(lane_id)` であれば明示的エイリアスを持つ lane の ID。
/// `None` の場合はリネーム不可（対象外、パース失敗、など）。
fn resolve_explicit_alias_lane<'a>(source: &str, word: &'a str) -> Option<&'a str> {
    let file = tdsl_parser::parse(source).ok()?;

    // AST をスキャンして、word と一致する alias を持つ LaneDecl を探す。
    // alias: Some(id) が word と一致する場合のみ明示的エイリアスとみなす。
    for stmt in &file.statements {
        if let tdsl_parser::ast::Statement::Lane(decl) = &stmt.node
            && let Some(alias) = &decl.alias
            && alias == word
        {
            return Some(word);
        }
    }

    None
}

/// `as <alias>` の識別子トークンの LSP Range を返す。
///
/// find_references / goto_definition 用の `lane.source_span` は宣言文全体を指すため
/// rename にそのまま流用すると宣言行全体を巻き込んで置換してしまう（#900）。
/// rename の宣言側編集は alias トークンの正確な範囲に限定する必要があるため、
/// AST から該当する `LaneDecl` の statement 範囲を特定し、その中で `as <alias>`
/// パターンを再検索する。
fn find_alias_decl_range(source: &str, alias: &str) -> Option<Range> {
    let file = tdsl_parser::parse(source).ok()?;
    let line_offsets = build_line_offsets(source);

    for stmt in &file.statements {
        if let tdsl_parser::ast::Statement::Lane(decl) = &stmt.node
            && decl.alias.as_deref() == Some(alias)
        {
            let (start, end) =
                find_keyword_token_range(source, stmt.span.start, stmt.span.end, "as", alias)?;
            return Some(Range {
                start: byte_offset_to_position(start, source, &line_offsets),
                end: byte_offset_to_position(end, source, &line_offsets),
            });
        }
    }

    None
}

/// IR から全 lane ID の一覧を取得する（衝突チェック用）。
fn collect_all_lane_ids(source: &str) -> Vec<String> {
    let Ok(file) = tdsl_parser::parse(source) else {
        return Vec::new();
    };
    let Ok(ir) = tdsl_core::lower::lower_static_with_source(&file, Some(source)) else {
        return Vec::new();
    };
    ir.lanes.iter().map(|l| l.id.clone()).collect()
}

// ---------------------------------------------------------------------------
// 公開インタフェース
// ---------------------------------------------------------------------------

/// prepareRename 要求を処理する。
///
/// カーソル位置のトークンが明示的 `as <alias>` を持つ lane ID であれば
/// そのトークンの LSP Range を返す。
/// - 対象外（`as` 省略 lane、lane 以外のトークン、パース失敗）は `None` を返す。
pub fn compute_prepare_rename(source: &str, position: Position) -> Option<Range> {
    let (word, word_range) = word_at_position(source, position)?;

    // IR で lane として存在するか確認（存在しなければ None）
    let file = tdsl_parser::parse(source).ok()?;
    let ir = tdsl_core::lower::lower_static_with_source(&file, Some(source)).ok()?;
    if !ir.lanes.iter().any(|l| l.id == word) {
        return None;
    }

    // AST で明示的 alias を持つか確認（MVP: alias なし lane は拒否）
    resolve_explicit_alias_lane(source, &word)?;

    Some(word_range)
}

/// rename 要求を処理して WorkspaceEdit を返す。
///
/// - `new_name` が slug 規則に違反する場合は `Err(message)` を返す。
/// - カーソル下のトークンがリネーム対象外（`as` 省略 lane など）の場合も `Err` を返す。
/// - 新名称が既存の他の lane ID と衝突する場合は `Err` を返す。
/// - 成功時は全参照位置（宣言含む）を `new_name` に置換する `WorkspaceEdit` を返す。
pub fn compute_rename(
    source: &str,
    position: Position,
    new_name: &str,
    uri: &Url,
) -> Result<WorkspaceEdit, String> {
    // slug 規則の検証
    if !is_valid_slug(new_name) {
        return Err(format!(
            "新名称 '{new_name}' は lane ID として無効です（ASCII英数字とアンダースコアのみ許可、空不可）"
        ));
    }

    // カーソル下のトークンを取得
    let (word, _word_range) = word_at_position(source, position)
        .ok_or_else(|| "カーソル位置にトークンが見つかりません".to_string())?;

    // IR で lane として存在するか確認
    let all_lane_ids = collect_all_lane_ids(source);
    if !all_lane_ids.iter().any(|id| id == &word) {
        return Err(format!("'{word}' は lane ID として存在しません"));
    }

    // MVP: 明示的 alias を持つ lane のみ対象
    resolve_explicit_alias_lane(source, &word).ok_or_else(|| {
        format!("'{word}' は `as` 省略 lane のためリネームできません（MVP スコープ外）")
    })?;

    // 衝突チェック: 新名称が既存の他の lane ID と一致しないか
    // 自分自身との一致（same name rename）はエラーとしない
    if new_name != word && all_lane_ids.iter().any(|id| id == new_name) {
        return Err(format!(
            "lane ID '{new_name}' は既に存在します。別の名称を指定してください"
        ));
    }

    // 宣言を除く全参照位置を取得（宣言側は alias トークン範囲を別途計算する。#900）
    let reference_locations = compute_references(source, position, false, uri)
        .ok_or_else(|| format!("'{word}' の参照位置を取得できませんでした"))?;

    // 宣言側は `as <alias>` トークンの正確な範囲のみを置換する（宣言行全体を巻き込まない）
    let decl_range = find_alias_decl_range(source, &word)
        .ok_or_else(|| format!("'{word}' の宣言位置を特定できませんでした"))?;

    let mut text_edits: Vec<TextEdit> = Vec::with_capacity(reference_locations.len() + 1);
    text_edits.push(TextEdit {
        range: decl_range,
        new_text: new_name.to_string(),
    });
    text_edits.extend(reference_locations.into_iter().map(|loc| TextEdit {
        range: loc.range,
        new_text: new_name.to_string(),
    }));

    // WorkspaceEdit を構築（URI → TextEdit 一覧）
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), text_edits);

    Ok(WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uri() -> Url {
        Url::parse("file:///test.tdsl").unwrap()
    }

    /// `WorkspaceEdit` の TextEdit 一覧を `source` に適用した結果を返す。
    ///
    /// LSP の `Position` は UTF-16 コードユニット単位の列オフセットを使うため、
    /// 各行を UTF-16 単位に変換してから置換範囲を求める。適用順は後方から行い、
    /// 前方の置換によるオフセットずれを避ける。
    fn apply_text_edits(source: &str, edits: &[TextEdit]) -> String {
        let lines: Vec<&str> = source.split('\n').collect();
        // UTF-16 単位の列 → バイトオフセットへの変換ヘルパー
        fn utf16_col_to_byte(line: &str, utf16_col: usize) -> usize {
            let mut utf16_count = 0usize;
            for (byte_idx, ch) in line.char_indices() {
                if utf16_count >= utf16_col {
                    return byte_idx;
                }
                utf16_count += ch.len_utf16();
            }
            line.len()
        }

        // 行番号でソートし、後方の行から先に適用することで前方の編集が
        // 後続編集のオフセットに影響しないようにする。
        let mut sorted_edits: Vec<&TextEdit> = edits.iter().collect();
        sorted_edits.sort_by(|a, b| {
            b.range
                .start
                .line
                .cmp(&a.range.start.line)
                .then(b.range.start.character.cmp(&a.range.start.character))
        });

        let mut result_lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();

        for edit in sorted_edits {
            let line_idx = edit.range.start.line as usize;
            let line = &result_lines[line_idx];
            let start_byte = utf16_col_to_byte(line, edit.range.start.character as usize);
            let end_byte = utf16_col_to_byte(line, edit.range.end.character as usize);
            let mut new_line = String::new();
            new_line.push_str(&line[..start_byte]);
            new_line.push_str(&edit.new_text);
            new_line.push_str(&line[end_byte..]);
            result_lines[line_idx] = new_line;
        }

        result_lines.join("\n")
    }

    const MINI_SRC: &str = concat!(
        "timeline \"test\" { title \"test\"; unit year; range 0..2000; calendar proleptic_gregorian; }\n",
        "lane \"漢\" as han { kind dynasty; order 10; }\n",
        "lane \"Wei\" as wei { kind dynasty; order 20; }\n",
        "span han 100..200 \"foo\" {};\n",
        "event han 150 \"bar\" {};\n",
    );

    // ── compute_prepare_rename ────────────────────────────────────────────

    /// 明示的 `as han` を持つ lane ID 上で Range を返す
    #[test]
    fn prepare_rename_explicit_alias_returns_range() {
        // lane 宣言行（0-based: 1）の "han"
        // `lane "漢" as han { ...`
        // l=0,a=1,n=2,e=3,' '=4,'"'=5,'漢'=6,'"'=7,' '=8,'a'=9,'s'=10,' '=11,'h'=12,'a'=13,'n'=14
        let pos = Position {
            line: 1,
            character: 12,
        };
        let result = compute_prepare_rename(MINI_SRC, pos);
        assert!(
            result.is_some(),
            "明示的エイリアスの lane は Some(Range) を返す"
        );
        let range = result.unwrap();
        assert_eq!(range.start.line, 1, "宣言行（0-based: 1）");
        assert_eq!(range.start.character, 12, "han の開始位置");
        assert_eq!(range.end.character, 15, "han の終了位置");
    }

    /// span 上の lane 参照（han）からでも prepare_rename が成功する
    #[test]
    fn prepare_rename_from_reference_position() {
        // span 行（0-based: 3）の "han": `span han ...`
        // s=0,p=1,a=2,n=3,' '=4,'h'=5,'a'=6,'n'=7
        let pos = Position {
            line: 3,
            character: 5,
        };
        let result = compute_prepare_rename(MINI_SRC, pos);
        assert!(result.is_some(), "参照位置からも prepare_rename 成功");
    }

    /// timeline キーワード上では None を返す
    #[test]
    fn prepare_rename_non_lane_token_returns_none() {
        let pos = Position {
            line: 0,
            character: 0,
        };
        let result = compute_prepare_rename(MINI_SRC, pos);
        assert!(result.is_none(), "非 lane ID は None を返す");
    }

    /// `as` 省略 lane では None を返す（MVP スコープ外）
    #[test]
    fn prepare_rename_auto_slug_lane_returns_none() {
        let src = concat!(
            "timeline \"test\" { title \"test\"; unit year; range 0..2000; calendar proleptic_gregorian; }\n",
            "lane \"emperor\" { kind custom; order 1; }\n",
            "span emperor 100..200 \"foo\" {};\n",
        );
        // `as` 省略 lane のスラッグ "emperor" の位置（span 行、0-based: 2）
        let pos = Position {
            line: 2,
            character: 5,
        };
        let result = compute_prepare_rename(src, pos);
        assert!(result.is_none(), "`as` 省略 lane は None を返す");
    }

    // ── compute_rename ────────────────────────────────────────────────────

    /// 正常系: han → han2 にリネームして2箇所（宣言+span参照+event参照 = 3件）更新
    #[test]
    fn rename_success_returns_workspace_edit() {
        // span 行（0-based: 3）の "han"
        let pos = Position {
            line: 3,
            character: 5,
        };
        let uri = test_uri();
        let result = compute_rename(MINI_SRC, pos, "han2", &uri);
        assert!(result.is_ok(), "リネーム成功: {result:?}");
        let edit = result.unwrap();
        let changes = edit.changes.unwrap();
        let edits = changes.get(&uri).unwrap();
        // han の参照: span(1) + event(1) + 宣言(1) = 3件
        assert_eq!(edits.len(), 3, "3件の TextEdit が生成される: {edits:?}");
        assert!(
            edits.iter().all(|e| e.new_text == "han2"),
            "全 TextEdit が 'han2' を new_text として持つ"
        );
    }

    /// 無効な slug（空文字列）はエラーを返す
    #[test]
    fn rename_invalid_slug_empty_returns_err() {
        let pos = Position {
            line: 3,
            character: 5,
        };
        let result = compute_rename(MINI_SRC, pos, "", &test_uri());
        assert!(result.is_err(), "空文字列は Err を返す");
    }

    /// 無効な slug（ハイフン含む）はエラーを返す
    #[test]
    fn rename_invalid_slug_with_hyphen_returns_err() {
        let pos = Position {
            line: 3,
            character: 5,
        };
        let result = compute_rename(MINI_SRC, pos, "han-new", &test_uri());
        assert!(result.is_err(), "ハイフン含む slug は Err を返す");
    }

    /// 既存の別 lane ID への衝突はエラーを返す
    #[test]
    fn rename_collision_with_existing_lane_returns_err() {
        // han → wei（既存）
        let pos = Position {
            line: 3,
            character: 5,
        };
        let result = compute_rename(MINI_SRC, pos, "wei", &test_uri());
        assert!(result.is_err(), "既存 lane ID への衝突は Err を返す");
        let msg = result.unwrap_err();
        assert!(msg.contains("wei"), "エラーメッセージに衝突名を含む: {msg}");
    }

    /// 同名へのリネーム（no-op）はエラーにしない
    #[test]
    fn rename_same_name_noop_is_ok() {
        let pos = Position {
            line: 3,
            character: 5,
        };
        let result = compute_rename(MINI_SRC, pos, "han", &test_uri());
        assert!(result.is_ok(), "同名リネームは Ok を返す: {result:?}");
    }

    /// `as` 省略 lane はエラーを返す（MVP スコープ外）
    #[test]
    fn rename_auto_slug_lane_returns_err() {
        let src = concat!(
            "timeline \"test\" { title \"test\"; unit year; range 0..2000; calendar proleptic_gregorian; }\n",
            "lane \"emperor\" { kind custom; order 1; }\n",
            "span emperor 100..200 \"foo\" {};\n",
        );
        let pos = Position {
            line: 2,
            character: 5,
        };
        let result = compute_rename(src, pos, "emp2", &test_uri());
        assert!(result.is_err(), "`as` 省略 lane は Err を返す");
    }

    /// lane 以外のトークン上では Err を返す
    #[test]
    fn rename_non_lane_token_returns_err() {
        // "timeline" の位置
        let pos = Position {
            line: 0,
            character: 0,
        };
        let result = compute_rename(MINI_SRC, pos, "new_id", &test_uri());
        assert!(result.is_err(), "lane 以外のトークンは Err を返す");
    }

    /// #900 回帰テスト: 宣言側の編集が `as <alias>` トークンのみに限定され、
    /// 宣言行全体（`kind` 等の他の属性含む）を巻き込まないこと。
    /// 適用後のソースを再パース・lowering して構文的にも意味的にも正しいことを検証する
    /// （編集件数と new_text だけを見る旧テストではこのバグを検出できなかった）。
    #[test]
    fn rename_applies_cleanly_and_preserves_declaration_attributes() {
        let src = concat!(
            "timeline \"T\" { unit year; range 0..100; }\n",
            "lane \"A\" as a { kind custom; }\n",
            "event a 10 \"E\" {};\n",
        );
        // event 行（0-based: 2）の "a" 上にカーソル
        let pos = Position {
            line: 2,
            character: 6,
        };
        let uri = test_uri();
        let edit = compute_rename(src, pos, "a2", &uri).expect("リネーム成功");
        let changes = edit.changes.expect("changes が設定されている");
        let edits = changes.get(&uri).expect("URI に対する編集がある");

        let applied = apply_text_edits(src, edits);

        let expected = concat!(
            "timeline \"T\" { unit year; range 0..100; }\n",
            "lane \"A\" as a2 { kind custom; }\n",
            "event a2 10 \"E\" {};\n",
        );
        assert_eq!(
            applied, expected,
            "宣言行の `kind custom;` 等の属性が保持され、alias のみ置換される"
        );

        // 適用後のソースが再パース・lowering可能であることを確認する
        let file = tdsl_parser::parse(&applied).expect("リネーム後もパース可能");
        let ir = tdsl_core::lower::lower_static_with_source(&file, Some(&applied))
            .expect("リネーム後も lowering 可能");
        assert!(
            ir.lanes.iter().any(|l| l.id == "a2"),
            "新 lane ID 'a2' が IR に存在する"
        );
        assert!(
            !ir.lanes.iter().any(|l| l.id == "a"),
            "旧 lane ID 'a' は IR に残っていない"
        );
        assert_eq!(ir.lanes[0].kind, "custom", "kind 属性が保持されている");
    }

    /// #900 回帰テスト: 複数行にまたがる lane 宣言・日本語ラベルを含む場合でも
    /// UTF-16 オフセット計算が崩れず、宣言側の編集が alias トークンのみに限定される。
    #[test]
    fn rename_multiline_declaration_with_japanese_label_preserves_attributes() {
        let src = concat!(
            "timeline \"T\" { unit year; range 0..2000; }\n",
            "lane \"漢王朝\" as han {\n",
            "  kind dynasty;\n",
            "  order 10;\n",
            "}\n",
            "span han 100..200 \"foo\" {};\n",
        );
        // span 行（0-based: 5）の "han" 上にカーソル
        let pos = Position {
            line: 5,
            character: 5,
        };
        let uri = test_uri();
        let edit = compute_rename(src, pos, "han2", &uri).expect("リネーム成功");
        let changes = edit.changes.expect("changes が設定されている");
        let edits = changes.get(&uri).expect("URI に対する編集がある");

        let applied = apply_text_edits(src, edits);

        let expected = concat!(
            "timeline \"T\" { unit year; range 0..2000; }\n",
            "lane \"漢王朝\" as han2 {\n",
            "  kind dynasty;\n",
            "  order 10;\n",
            "}\n",
            "span han2 100..200 \"foo\" {};\n",
        );
        assert_eq!(
            applied, expected,
            "複数行宣言でも `kind` / `order` が保持され alias のみ置換される"
        );

        let file = tdsl_parser::parse(&applied).expect("リネーム後もパース可能");
        let ir = tdsl_core::lower::lower_static_with_source(&file, Some(&applied))
            .expect("リネーム後も lowering 可能");
        assert!(ir.lanes.iter().any(|l| l.id == "han2"));
        assert_eq!(ir.lanes[0].order, 10, "order 属性が保持されている");
    }

    /// is_valid_slug のユニットテスト
    #[test]
    fn is_valid_slug_accepts_valid_names() {
        assert!(is_valid_slug("han"));
        assert!(is_valid_slug("Han_dynasty"));
        assert!(is_valid_slug("lane_1"));
        assert!(is_valid_slug("A"));
        assert!(is_valid_slug("abc123"));
    }

    #[test]
    fn is_valid_slug_rejects_invalid_names() {
        assert!(!is_valid_slug(""));
        assert!(!is_valid_slug("han-new"));
        assert!(!is_valid_slug("han new"));
        assert!(!is_valid_slug("漢"));
        assert!(!is_valid_slug("han.new"));
    }
}
