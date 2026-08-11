/// Source span for error reporting.
// `usize` 2 つの値型なので Copy にしておく。lowering がエラーへ位置を
// 添えるとき（#760）に共有参照から取り出すため。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// 時刻リテラル。年・月・日・時・分・秒・オフセットの精度を保持する。
///
/// `Year(y)` は `YYYY` または `-YYYY`（紀元前）、
/// `YearMonth(y, m)` は `YYYY-MM`、
/// `Date(y, m, d)` は `YYYY-MM-DD`、
/// `DateTime(y, m, d, hh, mm)` は `YYYY-MM-DDTHH:MM` に対応する。
///
/// 秒・オフセット付き variant（ADR 0003）:
/// `DateTimeSecond(y, m, d, hh, mm, ss)` は `YYYY-MM-DDTHH:MM:SS`、
/// `DateTimeOffset(y, m, d, hh, mm, offset_min)` は `YYYY-MM-DDTHH:MM(Z|±HH:MM)`、
/// `DateTimeSecondOffset(y, m, d, hh, mm, ss, offset_min)` は
/// `YYYY-MM-DDTHH:MM:SS(Z|±HH:MM)` に対応する。
/// `offset_min` は分単位（例: `+09:00` → `540`、`-05:00` → `-300`、`Z` → `0`）。
/// offset の有無自体が意味を持つ（ADR 0003 D2/D3）ため、`DateTime`/`DateTimeSecond`
/// （offsetなし）と `DateTimeOffset`/`DateTimeSecondOffset`（offsetあり、`Z` も offset
/// あり=0分として扱う）は明確に区別される。offset 付き値同士の正規化比較・
/// offsetなしとの混在比較エラーは lowering（`tdsl-core`）の責務（ADR 0003 D2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeValue {
    Year(i64),
    YearMonth(i64, u8),
    Date(i64, u8, u8),
    DateTime(i64, u8, u8, u8, u8),
    /// `YYYY-MM-DDTHH:MM:SS`（offsetなし）。
    DateTimeSecond(i64, u8, u8, u8, u8, u8),
    /// `YYYY-MM-DDTHH:MM(Z|±HH:MM)`。最後の `i16` は offset（分単位）。
    DateTimeOffset(i64, u8, u8, u8, u8, i16),
    /// `YYYY-MM-DDTHH:MM:SS(Z|±HH:MM)`。最後の `i16` は offset（分単位）。
    DateTimeSecondOffset(i64, u8, u8, u8, u8, u8, i16),
}

impl TimeValue {
    /// 年部分のみ取り出す（month/day 精度の情報は失われる）。
    pub fn year(&self) -> i64 {
        match self {
            TimeValue::Year(y) => *y,
            TimeValue::YearMonth(y, _) => *y,
            TimeValue::Date(y, _, _)
            | TimeValue::DateTime(y, _, _, _, _)
            | TimeValue::DateTimeSecond(y, _, _, _, _, _)
            | TimeValue::DateTimeOffset(y, _, _, _, _, _)
            | TimeValue::DateTimeSecondOffset(y, _, _, _, _, _, _) => *y,
        }
    }

    pub fn month(&self) -> Option<u8> {
        match self {
            TimeValue::Year(_) => None,
            TimeValue::YearMonth(_, m) => Some(*m),
            TimeValue::Date(_, m, _)
            | TimeValue::DateTime(_, m, _, _, _)
            | TimeValue::DateTimeSecond(_, m, _, _, _, _)
            | TimeValue::DateTimeOffset(_, m, _, _, _, _)
            | TimeValue::DateTimeSecondOffset(_, m, _, _, _, _, _) => Some(*m),
        }
    }

    pub fn day(&self) -> Option<u8> {
        match self {
            TimeValue::Year(_) | TimeValue::YearMonth(_, _) => None,
            TimeValue::Date(_, _, d)
            | TimeValue::DateTime(_, _, d, _, _)
            | TimeValue::DateTimeSecond(_, _, d, _, _, _)
            | TimeValue::DateTimeOffset(_, _, d, _, _, _)
            | TimeValue::DateTimeSecondOffset(_, _, d, _, _, _, _) => Some(*d),
        }
    }

    pub fn hour(&self) -> Option<u8> {
        match self {
            TimeValue::DateTime(_, _, _, h, _)
            | TimeValue::DateTimeSecond(_, _, _, h, _, _)
            | TimeValue::DateTimeOffset(_, _, _, h, _, _)
            | TimeValue::DateTimeSecondOffset(_, _, _, h, _, _, _) => Some(*h),
            _ => None,
        }
    }

    pub fn minute(&self) -> Option<u8> {
        match self {
            TimeValue::DateTime(_, _, _, _, m)
            | TimeValue::DateTimeSecond(_, _, _, _, m, _)
            | TimeValue::DateTimeOffset(_, _, _, _, m, _)
            | TimeValue::DateTimeSecondOffset(_, _, _, _, m, _, _) => Some(*m),
            _ => None,
        }
    }

    /// 秒部分。秒精度を持たない値では `None`。
    pub fn second(&self) -> Option<u8> {
        match self {
            TimeValue::DateTimeSecond(_, _, _, _, _, second)
            | TimeValue::DateTimeSecondOffset(_, _, _, _, _, second, _) => Some(*second),
            _ => None,
        }
    }

    /// UTC からのオフセット（分単位）。オフセットを持たない civil time では `None`。
    pub fn offset_minutes(&self) -> Option<i16> {
        match self {
            TimeValue::DateTimeOffset(_, _, _, _, _, offset)
            | TimeValue::DateTimeSecondOffset(_, _, _, _, _, _, offset) => Some(*offset),
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
            TimeValue::Date(y, m, d)
            | TimeValue::DateTime(y, m, d, _, _)
            | TimeValue::DateTimeSecond(y, m, d, _, _, _)
            | TimeValue::DateTimeOffset(y, m, d, _, _, _)
            | TimeValue::DateTimeSecondOffset(y, m, d, _, _, _, _) => (*y, *m, *d),
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
            TimeValue::DateTimeSecond(y, m, d, h, min, second) => {
                write!(f, "{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{second:02}")
            }
            TimeValue::DateTimeOffset(y, m, d, h, min, offset) => {
                write!(f, "{y:04}-{m:02}-{d:02}T{h:02}:{min:02}")?;
                write_offset(f, *offset)
            }
            TimeValue::DateTimeSecondOffset(y, m, d, h, min, second, offset) => {
                write!(f, "{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{second:02}")?;
                write_offset(f, *offset)
            }
        }
    }
}

/// offset（分単位）を `Z`（0分）または `±HH:MM` として書き出す内部ヘルパ。
fn write_offset(f: &mut std::fmt::Formatter<'_>, offset_minutes: i16) -> std::fmt::Result {
    if offset_minutes == 0 {
        write!(f, "Z")
    } else {
        let sign = if offset_minutes < 0 { '-' } else { '+' };
        let abs = offset_minutes.unsigned_abs();
        write!(f, "{sign}{:02}:{:02}", abs / 60, abs % 60)
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
    /// lane の色（`color "#4a9eff";`）。未指定なら `None`。
    ///
    /// レンダラは lane の並び順から色を機械的に割り当てるため、lane を 1 つ
    /// 足したり `order` を変えると**既存 lane の色が全部ずれる**。
    /// バージョン管理下で長期運用する `.tdsl` では、色の非決定性が
    /// そのまま図の非決定性になる（#747）。
    pub color: Option<String>,
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

impl std::fmt::Display for SourceRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.prefix, self.qid)
    }
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
    /// `.year` 等の accessor。未指定なら `None`（年精度として扱う）。
    pub accessor: Option<ClaimAccessor>,
    /// Year offset applied after claim resolution (e.g. `+1`, `-30`).
    pub offset: Option<i32>,
}

/// `claim(P571).year` の `.year` にあたる accessor。
///
/// 以前は素の `String` で、`grammar.pest` の `claim_accessor = { "." ~ ident }` が
/// 任意の ident を受理していた。未知の accessor は lowering の
/// `eval_claim_expr` が黙って `None` にするため、`.yaer` のような typo が
/// パースを通り、「required `start`/`end` could not be resolved」という
/// **原因を誤誘導する汎用 warning** だけを残してアイテムが消えていた（#758）。
///
/// 有効値を型で閉じ、未知の accessor はパース時に拒否する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimAccessor {
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

impl ClaimAccessor {
    /// 受理する accessor 名の一覧。エラーメッセージと解析の唯一の出所にする
    /// （2 箇所に書くと、追加したとき片方だけ更新される）。
    pub const NAMES: [&'static str; 6] = ["year", "month", "day", "hour", "minute", "second"];

    /// accessor 名から解析する。未知の名前は `None`。
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "year" => Some(Self::Year),
            "month" => Some(Self::Month),
            "day" => Some(Self::Day),
            "hour" => Some(Self::Hour),
            "minute" => Some(Self::Minute),
            "second" => Some(Self::Second),
            _ => None,
        }
    }
}

impl ClaimAccessor {
    /// accessor 名を返す。`Display` と `from_name` が同じ表記を使うことで、
    /// フォーマッタの出力を再パースできる（往復性）。
    pub fn name(self) -> &'static str {
        match self {
            Self::Year => "year",
            Self::Month => "month",
            Self::Day => "day",
            Self::Hour => "hour",
            Self::Minute => "minute",
            Self::Second => "second",
        }
    }
}

impl std::fmt::Display for ClaimAccessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
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
