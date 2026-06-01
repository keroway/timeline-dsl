use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Syntax error: {0}")]
    Syntax(#[from] pest::error::Error<crate::Rule>),

    #[error("Invalid integer at {location}: {value}")]
    InvalidInt { value: String, location: String },

    #[error("Unknown re-import policy: {0}")]
    UnknownPolicy(String),

    #[error("Unknown map target type '{0}' (expected one of: span, event, event_range)")]
    UnknownTargetType(String),

    #[error("Unexpected rule {rule} at {location}")]
    UnexpectedRule { rule: String, location: String },

    #[error("Invalid month at {location}: {value} (expected 1-12)")]
    InvalidMonth { value: u32, location: String },

    #[error("Invalid day at {location}: {value} (expected 1-31)")]
    InvalidDay { value: u32, location: String },
}

/// DSL ソース内の位置情報（1-based 行番号・列番号）。
/// LSP や WASM バインディングで診断位置を返すために使用する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseErrorLoc {
    /// 開始行（1-based）。
    pub line: u32,
    /// 開始列（1-based、バイト単位）。
    pub col: u32,
    /// 終了行（1-based）。開始と同じ行のことが多い。
    pub end_line: u32,
    /// 終了列（1-based、バイト単位）。
    pub end_col: u32,
}

impl ParseError {
    /// パースエラーのソース位置を返す。
    ///
    /// - `Syntax` variant は pest の `line_col` から直接取得する。
    /// - バイトオフセット variant（`InvalidInt` / `InvalidMonth` / `InvalidDay` /
    ///   `UnexpectedRule`）は `location` フィールドの `"start:end"` 文字列と
    ///   `src` を使ってバイトオフセット→行列に変換する。
    /// - `UnknownPolicy` / `UnknownTargetType` は位置情報を持たないため `None`。
    pub fn source_location(&self, src: &str) -> Option<ParseErrorLoc> {
        match self {
            ParseError::Syntax(e) => {
                use pest::error::LineColLocation;
                match e.line_col {
                    LineColLocation::Pos((line, col)) => Some(ParseErrorLoc {
                        line: line as u32,
                        col: col as u32,
                        end_line: line as u32,
                        end_col: col as u32,
                    }),
                    LineColLocation::Span((sl, sc), (el, ec)) => Some(ParseErrorLoc {
                        line: sl as u32,
                        col: sc as u32,
                        end_line: el as u32,
                        end_col: ec as u32,
                    }),
                }
            }
            ParseError::InvalidInt { location, .. }
            | ParseError::UnexpectedRule { location, .. }
            | ParseError::InvalidMonth { location, .. }
            | ParseError::InvalidDay { location, .. } => byte_range_to_loc(location, src),
            ParseError::UnknownPolicy(_) | ParseError::UnknownTargetType(_) => None,
        }
    }
}

/// `"start:end"` 形式のバイトオフセット文字列からソース位置に変換する（内部ヘルパ）。
fn byte_range_to_loc(location: &str, src: &str) -> Option<ParseErrorLoc> {
    let (start_str, end_str) = location.split_once(':')?;
    let start_byte: usize = start_str.trim().parse().ok()?;
    let end_byte: usize = end_str.trim().parse().ok()?;

    let (start_line, start_col) = byte_offset_to_line_col(src, start_byte);
    let (end_line, end_col) = byte_offset_to_line_col(src, end_byte);

    Some(ParseErrorLoc {
        line: start_line,
        col: start_col,
        end_line,
        end_col,
    })
}

/// バイトオフセットを 1-based の (line, col) に変換する。
///
/// pest の span はバイト単位かつ char 境界に揃っているため `src` のスライスは安全。
/// LSP など、AST ノードの `Span`（バイトオフセット）から行・列を求める用途でも再利用する。
pub fn byte_offset_to_line_col(src: &str, offset: usize) -> (u32, u32) {
    // オフセットがソース長を超えていたら末尾に丸める
    let offset = offset.min(src.len());
    let before = &src[..offset];
    let line = (before.chars().filter(|&c| c == '\n').count() + 1) as u32;
    let col = (before.rfind('\n').map_or(offset, |pos| offset - pos - 1) + 1) as u32;
    (line, col)
}
