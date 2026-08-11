use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoweringError {
    #[error("Parse error: {0}")]
    Parse(#[from] tdsl_parser::error::ParseError),

    #[error("Unknown lane reference: {0}")]
    UnknownLane(String),

    #[error("Duplicate lane alias: {0}")]
    DuplicateLane(String),

    #[error("Duplicate item id: {0}")]
    DuplicateItemId(String),

    /// `policy field_priority` で ID が衝突したが、item の型（span / event /
    /// event_range）が食い違っている。
    ///
    /// 以前は incoming が黙って既存を丸ごと置換しており、`label manual` 等の
    /// 設定も無視されて「手動データを守る」という field_priority の意図と
    /// 逆の結果になっていた（#762）。
    #[error(
        "Item id `{id}` has conflicting types under `policy field_priority`: existing is `{existing}`, incoming is `{incoming}` (field-level merge is only defined between items of the same type)"
    )]
    FieldPriorityTypeMismatch {
        /// 衝突した item の ID。
        id: String,
        /// 既存アイテムの型名。
        existing: &'static str,
        /// 取り込もうとしたアイテムの型名。
        incoming: &'static str,
    },

    #[error("No timeline block found")]
    NoTimeline,

    #[error("Multiple timeline blocks found")]
    MultipleTimelines,

    #[error("Unknown timeline unit: {value} (expected one of: {expected})")]
    UnknownTimelineUnit { value: String, expected: String },

    #[cfg(feature = "wikidata")]
    #[error("Wikidata error: {0}")]
    Wikidata(#[from] tdsl_wikidata::WikidataError),

    #[error("Unresolved import reference: {0}")]
    UnresolvedImport(String),

    #[error("Unresolved entity key: {0}")]
    UnresolvedEntity(String),

    #[error("Map references unknown lane: {0}")]
    UnknownMappedLane(String),

    /// 同じ alias の `import` ブロックが 2 つある。
    ///
    /// 以前は 2 つ目が 1 つ目のエンティティ群を `HashMap::insert` で黙って
    /// 置換していた。`as` を省略すると alias は `source_type`（通常
    /// `wikidata`）になるため、**alias 省略の import を 2 つ書くだけで踏む**。
    /// lane / template は同条件をエラーにしており、import だけが
    /// silent fallback だった（#761）。
    #[error(
        "Duplicate import alias: {0} (use `import <QID> as <alias>` to give each import block a distinct alias)"
    )]
    DuplicateImportAlias(String),

    #[error("Duplicate template alias: {0}")]
    DuplicateTemplate(String),

    #[error("Unknown template reference: {0}")]
    UnknownTemplate(String),

    #[error("Invalid item link URL: {0} (expected http:// or https:// URL)")]
    InvalidItemLink(String),

    #[error("Invalid item color value: {0}")]
    InvalidItemColor(String),

    /// offset付きtime valueとoffsetなしtime valueの比較は曖昧なので明示エラーとする
    /// （ADR 0003 D2: 「オフセットなしは暗黙にUTCとはみなさない」/ CLAUDE.md "No silent fallback" 原則）。
    #[error(
        "Cannot compare a UTC-offset time value with a value that has no offset (author must make both sides consistent): {0} vs {1}"
    )]
    MixedOffsetComparison(String, String),
}

/// `LoweringError` に発生元のソース位置を添えたもの。
///
/// `LoweringError` 自体は全 variant が素の文字列で位置を持たず、CLI は
/// `to_string()` を join するだけだった。パースエラーは v1.14 で miette の
/// キャレット表示になった一方、lowering エラー（E101〜）はメッセージのみで、
/// 大きいファイルでは該当行を探す手段が無い（#760）。
///
/// **variant ごとにフィールドを足すのではなくラッパにした理由**: 生成箇所が
/// 24 箇所あり、そのすべてに span を配るとエラーの定義自体が肥大する。
/// 加えて `Display` を委譲すれば、`e.to_string()` で表示している既存の
/// 呼び出し元（WASM / LSP / CLI の一部）はそのまま動く。
#[derive(Debug, Error)]
// 元エラーの表示をそのまま使う。**位置情報を文字列に混ぜない** —
// 混ぜると `to_string()` を使う既存の呼び出し元の出力が変わってしまう。
// 位置は miette 表示側（CLI の `LoweringDiagnostic`）だけが使う。
#[error("{error}")]
pub struct SpannedLoweringError {
    /// 元のエラー。
    #[source]
    pub error: LoweringError,
    /// 発生元のバイト範囲。位置を特定できない場合（ファイル全体に対する
    /// `NoTimeline` 等）は `None`。**「不明」を「先頭」と偽らない。**
    pub span: Option<tdsl_parser::ast::Span>,
}

impl SpannedLoweringError {
    /// 位置付きで構築する。
    pub fn new(error: LoweringError, span: Option<tdsl_parser::ast::Span>) -> Self {
        Self { error, span }
    }
}

impl From<LoweringError> for SpannedLoweringError {
    fn from(error: LoweringError) -> Self {
        Self { error, span: None }
    }
}

impl LoweringError {
    /// `docs/error-catalog.md` に対応する安定した診断コードを返す。
    ///
    /// CI で特定の診断だけを許容/禁止できるようにするための識別子（#748）。
    /// **カタログの見出し（`### E101: …`）と 1 対 1 で対応させること。**
    /// 対応が崩れていないかは `error_codes_match_catalog` テストが検証する。
    ///
    /// `Parse` と `Wikidata` は他レイヤ由来のエラーを包んだものなので、
    /// lowering の E1xx 体系には含めない（`None` を返す）。
    pub fn code(&self) -> Option<&'static str> {
        Some(match self {
            Self::Parse(_) => return None,
            #[cfg(feature = "wikidata")]
            Self::Wikidata(_) => return None,
            Self::UnknownLane(_) => "E101",
            Self::DuplicateLane(_) => "E102",
            Self::DuplicateItemId(_) => "E103",
            Self::NoTimeline => "E104",
            Self::MultipleTimelines => "E105",
            Self::UnresolvedImport(_) => "E106",
            Self::UnresolvedEntity(_) => "E107",
            Self::UnknownMappedLane(_) => "E108",
            Self::DuplicateTemplate(_) => "E109",
            Self::UnknownTemplate(_) => "E110",
            Self::InvalidItemLink(_) => "E111",
            Self::InvalidItemColor(_) => "E112",
            Self::MixedOffsetComparison(_, _) => "E113",
            Self::FieldPriorityTypeMismatch { .. } => "E114",
            Self::DuplicateImportAlias(_) => "E115",
            // `UnknownTimelineUnit` は E1xx を割り当てていない（カタログにも
            // 節が無い）。コードを付けるならカタログ側の追記とセットで行う。
            Self::UnknownTimelineUnit { .. } => return None,
        })
    }
}

impl SpannedLoweringError {
    /// 元のエラーの診断コード。
    pub fn code(&self) -> Option<&'static str> {
        self.error.code()
    }
}
