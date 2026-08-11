use miette::{Diagnostic, NamedSource, SourceSpan};
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

    #[error("Unknown claim accessor '{value}' (expected one of: {expected})")]
    UnknownClaimAccessor {
        value: String,
        expected: String,
        /// バイト範囲 `"start:end"`。miette のキャレット表示に使う。
        location: String,
    },

    #[error("Unexpected rule {rule} at {location}")]
    UnexpectedRule { rule: String, location: String },

    #[error("Invalid month at {location}: {value} (expected 1-12)")]
    InvalidMonth { value: u32, location: String },

    #[error("Invalid day at {location}: {value} (expected 1-31)")]
    InvalidDay { value: u32, location: String },

    #[error("Invalid second at {location}: {value} (expected 0-59)")]
    InvalidSecond { value: u32, location: String },

    #[error("Invalid UTC offset at {location}: {value} (expected Z or -14:00 through +14:00)")]
    InvalidOffset { value: String, location: String },
}

/// miette の fancy レポート（キャレット付きスニペット）を生成するための診断ラッパー。
///
/// CLI 層でのみ使用する。ライブラリ API では `ParseError` を直接返す。
/// `source` テキストを添付することで miette がキャレット行を表示できる。
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(
    code(tdsl::parse_error),
    help("DSL 仕様書 docs/dsl-spec.md を確認してください")
)]
pub struct ParseDiagnostic {
    message: String,
    #[source_code]
    src: NamedSource<String>,
    #[label("ここに問題があります")]
    span: Option<SourceSpan>,
}

impl ParseDiagnostic {
    /// `ParseError` と DSL ソース文字列からキャレット付き診断を構築する。
    ///
    /// - `Syntax` variant: pest の `variant.message()` から簡潔な説明文のみを取得する。
    ///   位置・スニペットは miette のキャレット描画に委ねる（pest の整形文字列は使わない）。
    /// - バイトオフセット variant（`InvalidInt` 等）: `location` フィールドの `"start:end"` 文字列を使う。
    /// - 位置情報のない variant（`UnknownPolicy` 等）: スパンなしで表示する。
    pub fn from_parse_error(err: &ParseError, src: &str, filename: &str) -> Self {
        let message = match err {
            // pest の完全整形文字列（位置・スニペット込み）は使わず、
            // variant.message() で "expected ..." の一行説明だけを取り出す。
            ParseError::Syntax(pest_err) => {
                format!("構文エラー: {}", pest_err.variant.message())
            }
            other => other.to_string(),
        };
        let named_src = NamedSource::new(filename, src.to_owned());

        let span = Self::extract_span(err, src);

        ParseDiagnostic {
            message,
            src: named_src,
            span,
        }
    }

    fn extract_span(err: &ParseError, src: &str) -> Option<SourceSpan> {
        match err {
            ParseError::Syntax(pest_err) => {
                use pest::error::InputLocation;
                match pest_err.location {
                    InputLocation::Pos(offset) => {
                        // 単一位置: 長さ 1 のスパン（それ以上は文脈がないため）
                        let offset = offset.min(src.len().saturating_sub(1));
                        Some(SourceSpan::from((offset, 1usize)))
                    }
                    InputLocation::Span((start, end)) => {
                        let start = start.min(src.len());
                        let end = end.min(src.len());
                        let len = end.saturating_sub(start).max(1);
                        Some(SourceSpan::from((start, len)))
                    }
                }
            }
            ParseError::InvalidInt { location, .. }
            | ParseError::UnexpectedRule { location, .. }
            | ParseError::InvalidMonth { location, .. }
            | ParseError::InvalidDay { location, .. }
            | ParseError::InvalidSecond { location, .. }
            | ParseError::InvalidOffset { location, .. }
            | ParseError::UnknownClaimAccessor { location, .. } => {
                parse_byte_range_to_span(location, src)
            }
            ParseError::UnknownPolicy(_) | ParseError::UnknownTargetType(_) => None,
        }
    }

    /// `SourceSpan` を返す（テストおよびカスタム表示向け）。
    pub fn span(&self) -> Option<SourceSpan> {
        self.span
    }
}

/// `"start:end"` 形式のバイトオフセット文字列を `SourceSpan` に変換する（内部ヘルパ）。
fn parse_byte_range_to_span(location: &str, src: &str) -> Option<SourceSpan> {
    let (start_str, end_str) = location.split_once(':')?;
    let start_byte: usize = start_str.trim().parse().ok()?;
    let end_byte: usize = end_str.trim().parse().ok()?;
    let start = start_byte.min(src.len());
    let end = end_byte.min(src.len());
    let len = end.saturating_sub(start).max(1);
    Some(SourceSpan::from((start, len)))
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
            | ParseError::InvalidDay { location, .. }
            | ParseError::InvalidSecond { location, .. }
            | ParseError::InvalidOffset { location, .. }
            | ParseError::UnknownClaimAccessor { location, .. } => byte_range_to_loc(location, src),
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
