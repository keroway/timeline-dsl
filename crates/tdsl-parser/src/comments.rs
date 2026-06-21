//! DSL ソースからコメントを byte span 付きで収集する独立パス。
//!
//! grammar の `COMMENT` は silent rule（`_{}`）のため pest の解析木に現れない。
//! コメントを AST に保持するため、ここで生ソースを 1 度走査し、文字列リテラルを
//! 尊重しながら `//` 行コメントと `/* */` ブロックコメントを抽出する（#472）。
//!
//! 文字列リテラル内の `//` や `/*` はコメントとして扱わない。

use crate::ast::{Comment, CommentKind, Span, Spanned};

/// `source` 中の全コメントを出現順に収集して返す。
///
/// 各コメントには元ソース上の byte span と、行頭出現か（`own_line`）の情報が付く。
pub fn scan_comments(source: &str) -> Vec<Spanned<Comment>> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0;
    // 直近に出現した「コメント以外の」非空白文字より後に改行があったか。
    // BOF 直後は行頭扱い（true）。
    let mut at_line_start = true;

    while i < len {
        let c = bytes[i];
        match c {
            b'"' => {
                // 文字列リテラル: エスケープ（\）を考慮して終端 " までスキップ。
                at_line_start = false;
                i += 1;
                while i < len {
                    match bytes[i] {
                        b'\\' => i += 2,
                        b'"' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'/' => {
                // 行コメント: 改行直前まで。
                let start = i;
                let own_line = at_line_start;
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                let text = source[start..i].to_string();
                out.push(Spanned {
                    node: Comment {
                        kind: CommentKind::Line,
                        text,
                        own_line,
                    },
                    span: Span { start, end: i },
                });
                // 行コメントの後は必ず改行（または EOF）なので次は行頭。
                at_line_start = true;
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                // ブロックコメント: `*/` まで（複数行可）。
                let start = i;
                let own_line = at_line_start;
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                // 終端 `*/` を含める。未終端なら EOF まで（grammar 側で別途エラー）。
                let end = if i + 1 < len { i + 2 } else { len };
                let text = source[start..end].to_string();
                out.push(Spanned {
                    node: Comment {
                        kind: CommentKind::Block,
                        text,
                        own_line,
                    },
                    span: Span { start, end },
                });
                i = end;
                // ブロックコメント後に同一行で続くコードがあり得るため行頭扱いにしない。
                at_line_start = false;
            }
            b'\n' => {
                at_line_start = true;
                i += 1;
            }
            b' ' | b'\t' | b'\r' => {
                // 空白は at_line_start を変えない。
                i += 1;
            }
            _ => {
                at_line_start = false;
                i += 1;
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_line_and_block_comments() {
        let src = "// header\nlane \"A\" as a {} /* trailing */\n";
        let comments = scan_comments(src);
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].node.kind, CommentKind::Line);
        assert_eq!(comments[0].node.text, "// header");
        assert!(comments[0].node.own_line);
        assert_eq!(comments[1].node.kind, CommentKind::Block);
        assert_eq!(comments[1].node.text, "/* trailing */");
        assert!(!comments[1].node.own_line);
    }

    #[test]
    fn ignores_comment_markers_inside_strings() {
        let src = r#"event a 1 "not // a comment /* nor this */" {};"#;
        let comments = scan_comments(src);
        assert!(comments.is_empty());
    }

    #[test]
    fn handles_escaped_quote_in_string() {
        let src = r#"lane "he said \"//\"" as x {} // real"#;
        let comments = scan_comments(src);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].node.text, "// real");
        assert!(!comments[0].node.own_line);
    }

    #[test]
    fn multiline_block_comment_span() {
        let src = "/* a\n b */\nlane \"x\" {}";
        let comments = scan_comments(src);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].node.text, "/* a\n b */");
        assert!(comments[0].node.own_line);
        assert_eq!(comments[0].span.start, 0);
        assert_eq!(comments[0].span.end, 10);
    }

    #[test]
    fn own_line_false_for_trailing_line_comment() {
        let src = "lane \"x\" {} // tail\n// own\n";
        let comments = scan_comments(src);
        assert_eq!(comments.len(), 2);
        assert!(!comments[0].node.own_line);
        assert!(comments[1].node.own_line);
    }
}
