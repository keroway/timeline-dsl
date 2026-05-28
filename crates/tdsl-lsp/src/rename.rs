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

use crate::find_references::compute_references;
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

    // 全参照位置（宣言含む）を取得
    let locations = compute_references(source, position, true, uri)
        .ok_or_else(|| format!("'{word}' の参照位置を取得できませんでした"))?;

    // 各 Location を TextEdit に変換
    let text_edits: Vec<TextEdit> = locations
        .into_iter()
        .map(|loc| TextEdit {
            range: loc.range,
            new_text: new_name.to_string(),
        })
        .collect();

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
