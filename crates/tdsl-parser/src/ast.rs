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
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Timeline(TimelineBlock),
    Lane(LaneDecl),
    Span(SpanDecl),
    Event(EventDecl),
    EventRange(EventRangeDecl),
    Import(ImportBlock),
    Map(MapBlock),
    Template(TemplateBlock),
    Apply(ApplyBlock),
}

// ─── Timeline ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineBlock {
    pub name: String,
    pub title: Option<String>,
    pub unit: Option<String>,
    pub range: Option<RangeExpr>,
    pub calendar: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeExpr {
    pub start: i64,
    pub end: i64,
}

// ─── Lane ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct LaneDecl {
    pub label: String,
    pub alias: Option<String>,
    pub kind: Option<String>,
    pub order: Option<i64>,
}

// ─── Items ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SpanDecl {
    pub lane_ref: String,
    pub start: i64,
    pub end: i64,
    pub label: String,
    pub props: ItemProps,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventDecl {
    pub lane_ref: String,
    pub time: i64,
    pub label: String,
    pub props: ItemProps,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventRangeDecl {
    pub lane_ref: String,
    pub start: i64,
    pub end: i64,
    pub label: String,
    pub props: ItemProps,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItemProps {
    pub tags: Vec<String>,
    pub source: Option<SourceRef>,
    pub id: Option<String>,
    pub origin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRef {
    pub prefix: String,
    pub qid: String,
}

// ─── Import ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ImportBlock {
    pub source_type: String,
    pub alias: Option<String>,
    pub items: Vec<ImportItem>,
    pub policy: Option<ReimportPolicy>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportItem {
    Entity {
        qid: String,
        alias: Option<String>,
    },
    Query {
        query: String,
        alias: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldStrategy {
    Manual,
    Wikidata,
    Merge,
}

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapExpr {
    pub claim: ClaimCall,
    pub accessor: Option<String>,
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
