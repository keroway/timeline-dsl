/// Source span for error reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// A node annotated with its source location.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

/// Root of the AST.
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub statements: Vec<Spanned<Statement>>,
    /// 元ソースに含まれていたコメント（出現順、byte span 付き）。
    ///
    /// コメントは [`crate::parse`] 時に専用パスで収集され、文（statement）とは独立に
    /// 保持される。lowering では一切参照されないため IR には影響しない（#362 / #473）。
    /// フォーマッタ（[`crate::format_file`]）はこの情報を使ってコメントを再 emit する。
    pub comments: Vec<Spanned<Comment>>,
}

/// コメントの種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    /// `// ...` 行コメント。
    Line,
    /// `/* ... */` ブロックコメント（複数行可）。
    Block,
}

/// 1 個のコメント。区切り文字（`//` / `/* */`）を含む生テキストを保持する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub kind: CommentKind,
    /// 区切り文字を含むコメント全文（例: `// foo`、`/* foo */`）。
    pub text: String,
    /// 行頭（直前が改行または BOF で、間に空白以外が無い）に出現したコメントか。
    ///
    /// `true` の場合は独立行コメント（後続文の leading コメント）として、
    /// `false` の場合は同一行末尾の trailing コメントとして整形される。
    pub own_line: bool,
}

/// DSL の各トップレベル文（statement）に対応する enum。
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Timeline(TimelineBlock),
    Lane(LaneDecl),
    Group(GroupDecl),
    Span(SpanDecl),
    Event(EventDecl),
    EventRange(EventRangeDecl),
    Import(ImportBlock),
    Map(MapBlock),
    Template(TemplateBlock),
    Apply(ApplyBlock),
}

// ─── Timeline ───────────────────────────────────────────────

/// `timeline "名前" { ... }` ブロックのAST表現。
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineBlock {
    pub name: String,
    pub title: Option<String>,
    pub unit: Option<String>,
    pub range: Option<RangeExpr>,
    pub calendar: Option<String>,
    pub color_map: Vec<(String, String)>,
}

/// 時刻リテラル。年・月・日・時・分の精度を保持する。
///
/// `Year(y)` は `YYYY` または `-YYYY`（紀元前）、
/// `YearMonth(y, m)` は `YYYY-MM`、
/// `Date(y, m, d)` は `YYYY-MM-DD`、
/// `DateTime(y, m, d, hh, mm)` は `YYYY-MM-DDTHH:MM` に対応する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeValue {
    Year(i64),
    YearMonth(i64, u8),
    Date(i64, u8, u8),
    DateTime(i64, u8, u8, u8, u8),
}

impl TimeValue {
    /// 年部分のみ取り出す（month/day 精度の情報は失われる）。
    pub fn year(&self) -> i64 {
        match self {
            TimeValue::Year(y) => *y,
            TimeValue::YearMonth(y, _) => *y,
            TimeValue::Date(y, _, _) | TimeValue::DateTime(y, _, _, _, _) => *y,
        }
    }

    pub fn month(&self) -> Option<u8> {
        match self {
            TimeValue::Year(_) => None,
            TimeValue::YearMonth(_, m) => Some(*m),
            TimeValue::Date(_, m, _) | TimeValue::DateTime(_, m, _, _, _) => Some(*m),
        }
    }

    pub fn day(&self) -> Option<u8> {
        match self {
            TimeValue::Year(_) | TimeValue::YearMonth(_, _) => None,
            TimeValue::Date(_, _, d) | TimeValue::DateTime(_, _, d, _, _) => Some(*d),
        }
    }

    pub fn hour(&self) -> Option<u8> {
        match self {
            TimeValue::DateTime(_, _, _, h, _) => Some(*h),
            _ => None,
        }
    }

    pub fn minute(&self) -> Option<u8> {
        match self {
            TimeValue::DateTime(_, _, _, _, m) => Some(*m),
            _ => None,
        }
    }

    /// 比較用のタプル `(year, month_or_0, day_or_0)`。
    /// `Eq` の意味（精度の違いを保持）と整合させるため、`PartialOrd` 実装には依らず
    /// 呼び出し側でこの関数を使って明示的にタプル順序で比較する。
    pub fn to_sortable(&self) -> (i64, u8, u8) {
        match self {
            TimeValue::Year(y) => (*y, 0, 0),
            TimeValue::YearMonth(y, m) => (*y, *m, 0),
            TimeValue::Date(y, m, d) | TimeValue::DateTime(y, m, d, _, _) => (*y, *m, *d),
        }
    }
}

impl std::fmt::Display for TimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeValue::Year(y) => write!(f, "{y}"),
            TimeValue::YearMonth(y, m) => write!(f, "{y:04}-{m:02}"),
            TimeValue::Date(y, m, d) => write!(f, "{y:04}-{m:02}-{d:02}"),
            TimeValue::DateTime(y, m, d, h, min) => {
                write!(f, "{y:04}-{m:02}-{d:02}T{h:02}:{min:02}")
            }
        }
    }
}

/// `start..end` 形式の時間範囲式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeExpr {
    pub start: TimeValue,
    pub end: TimeValue,
}

// ─── Lane ───────────────────────────────────────────────────

/// `lane "ラベル" as id { ... }` 宣言のAST表現。
#[derive(Debug, Clone, PartialEq)]
pub struct LaneDecl {
    pub label: String,
    pub alias: Option<String>,
    pub kind: Option<String>,
    pub order: Option<i64>,
}

/// `group "名前" { lane ... }` 宣言のAST表現。
#[derive(Debug, Clone, PartialEq)]
pub struct GroupDecl {
    pub label: String,
    pub lanes: Vec<LaneDecl>,
}

// ─── Items ──────────────────────────────────────────────────

/// `span <lane> <start>..<end> "ラベル" { ... }` 宣言のAST表現。
///
/// `end` に `now` を書いた場合は `end_open = true` となり、`end` には
/// ビルド時点の現在年（UTC）が補完される（#550）。
#[derive(Debug, Clone, PartialEq)]
pub struct SpanDecl {
    pub lane_ref: String,
    pub start: TimeValue,
    pub end: TimeValue,
    pub end_open: bool,
    pub label: String,
    pub props: ItemProps,
}

/// `event <lane> <time> "ラベル" { ... }` 宣言のAST表現。
#[derive(Debug, Clone, PartialEq)]
pub struct EventDecl {
    pub lane_ref: String,
    pub time: TimeValue,
    pub label: String,
    pub props: ItemProps,
}

/// `event_range <lane> <start>..<end> "ラベル" { ... }` 宣言のAST表現。
///
/// `end` に `now` を書いた場合は `end_open = true`（#550）。
#[derive(Debug, Clone, PartialEq)]
pub struct EventRangeDecl {
    pub lane_ref: String,
    pub start: TimeValue,
    pub end: TimeValue,
    pub end_open: bool,
    pub label: String,
    pub props: ItemProps,
}

/// アイテム共通の省略可能プロパティ（`tags`, `source`, `id`, `origin`, `note`, `link`, `color`）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItemProps {
    pub tags: Vec<String>,
    pub source: Option<SourceRef>,
    pub id: Option<String>,
    pub origin: Option<String>,
    pub note: Option<String>,
    pub link: Option<String>,
    pub color: Option<String>,
}

/// `source <prefix>:<qid>` 形式の出典参照（例: `source wd:Q7209`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRef {
    pub prefix: String,
    pub qid: String,
}

// ─── Import ─────────────────────────────────────────────────

/// `import <source_type> as <alias> { ... }` ブロックのAST表現。
#[derive(Debug, Clone, PartialEq)]
pub struct ImportBlock {
    pub source_type: String,
    pub alias: Option<String>,
    pub items: Vec<ImportItem>,
    pub policy: Option<ReimportPolicy>,
}

/// `import` ブロック内の個別エントリ（`entity` または `query`）。
#[derive(Debug, Clone, PartialEq)]
pub enum ImportItem {
    /// `entity QXXX as alias` — 単一エンティティのインポート。
    Entity { qid: String, alias: Option<String> },
    /// `query "SPARQL" as alias` — SPARQL クエリで複数エンティティを一括インポート。
    Query {
        query: String,
        alias: Option<String>,
    },
}

/// フィールド別インポート優先度戦略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldStrategy {
    /// 手動設定値を優先する。
    Manual,
    /// Wikidata 取得値を優先する。
    Wikidata,
    /// 両方をマージする。
    Merge,
}

/// `policy field_priority { ... }` ブロックのAST表現。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldPriorityConfig {
    pub label: FieldStrategy,
    pub time: FieldStrategy,
    pub tags: FieldStrategy,
}

impl Default for FieldPriorityConfig {
    fn default() -> Self {
        Self {
            label: FieldStrategy::Manual,
            time: FieldStrategy::Wikidata,
            tags: FieldStrategy::Merge,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReimportPolicy {
    MergeBySource,
    OverwriteImported,
    KeepManual,
    FieldPriority(FieldPriorityConfig),
}

// ─── Template / Apply ───────────────────────────────────────

/// Named reusable map pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateBlock {
    pub name: String,
    pub alias: Option<String>,
    pub target_type: MapTargetType,
    pub props: Vec<MapProp>,
}

/// Applies a template to an import alias with optional overrides.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyBlock {
    pub template_alias: String,
    pub import_alias: String,
    /// Overriding props (currently only lane).
    pub overrides: Vec<MapProp>,
}

// ─── Map ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapTargetType {
    Span,
    Event,
    EventRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapBlock {
    pub source_ref: String,
    pub target_type: MapTargetType,
    pub props: Vec<MapProp>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MapProp {
    Lane(String),
    Start(MapExpr),
    End(MapExpr),
    Time(MapExpr),
    Label(LabelExpr),
    Tags(Vec<String>),
    Filter(FilterExpr),
    /// `expand claim(P39);` — expands multiple non-deprecated statements into separate items.
    Expand(ClaimCall),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapExpr {
    pub fallbacks: Vec<MapFallback>,
}

/// A single fallback element in a `??` chain: either a claim expression or an integer literal.
#[derive(Debug, Clone, PartialEq)]
pub enum MapFallback {
    Claim(ClaimExpr),
    Literal(i64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClaimExpr {
    pub claim: ClaimCall,
    /// Qualifier property to access (e.g. `"P580"` for `.qualifier(P580)`).
    /// When `Some`, the qualifier snak of the main claim is resolved instead of the mainsnak.
    pub qualifier: Option<String>,
    pub accessor: Option<String>,
    /// Year offset applied after claim resolution (e.g. `+1`, `-30`).
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimCall {
    pub property: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LabelExpr {
    pub fallbacks: Vec<LabelRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelRef {
    pub lang: String,
}

// ─── Filter expressions (for map `filter` clause) ───────────

#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpr {
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Compare {
        lhs: FilterOperand,
        op: CompareOp,
        rhs: FilterOperand,
    },
    StringMatch {
        lhs: LabelRef,
        op: StringMatchOp,
        rhs: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringMatchOp {
    Contains,
    StartsWith,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterOperand {
    Claim(ClaimExpr),
    Int(i64),
    Null,
}
