use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Supported timeline axis units.
///
/// IR JSON keeps serializing `Meta::unit` as a string for backward
/// compatibility; this enum is the internal value set used by lowering and
/// validation to reject typos instead of falling back silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineUnit {
    Year,
    Month,
    Day,
    /// Sub-day axis unit (#556): hour ticks, for timelines spanning hours to
    /// a few days (mission logs, incident timelines, event schedules).
    Hour,
    /// Sub-day axis unit (#556): minute ticks, for very short timelines.
    Minute,
    /// Sub-day axis unit (#614, ADR 0003): second ticks, for the shortest
    /// timelines (recordings, movement logs, sub-minute event schedules).
    Second,
}

impl TimelineUnit {
    pub const ALL: [Self; 6] = [
        Self::Year,
        Self::Month,
        Self::Day,
        Self::Hour,
        Self::Minute,
        Self::Second,
    ];

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "year" => Some(Self::Year),
            "month" => Some(Self::Month),
            "day" => Some(Self::Day),
            "hour" => Some(Self::Hour),
            "minute" => Some(Self::Minute),
            "second" => Some(Self::Second),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
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

/// Known lane kinds used for diagnostics and completions.
///
/// `custom` remains the escape hatch for user-defined lane categories. Unknown
/// explicit `kind` values are warnings rather than lowering errors so existing
/// semantic classifications do not become hard failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneKind {
    Custom,
    Dynasty,
    Person,
    Country,
    Event,
}

impl LaneKind {
    pub const ALL: [Self; 5] = [
        Self::Custom,
        Self::Dynasty,
        Self::Person,
        Self::Country,
        Self::Event,
    ];

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "custom" => Some(Self::Custom),
            "dynasty" => Some(Self::Dynasty),
            "person" => Some(Self::Person),
            "country" => Some(Self::Country),
            "event" => Some(Self::Event),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::Dynasty => "dynasty",
            Self::Person => "person",
            Self::Country => "country",
            Self::Event => "event",
        }
    }
}

pub fn supported_timeline_units() -> String {
    TimelineUnit::ALL
        .iter()
        .map(|u| u.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn known_lane_kinds() -> String {
    LaneKind::ALL
        .iter()
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// DSL ソース内のアイテム定義位置（1-based 行番号・列番号）。
/// `source_span` が付いていない場合はスキップして JSON に出力しない。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SourceSpan {
    /// 定義開始行（1-based）。
    pub line: u32,
    /// 定義開始列（1-based, バイト単位）。
    pub col_start: u32,
    /// 定義終了列（1-based, バイト単位）。`start` と同じ行の場合のみ有効。
    pub col_end: u32,
}

/// `.tdsl` ファイルをコンパイルした結果の正規中間表現（JSON 直列化対象）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimelineIr {
    /// 年表のメタデータ（タイトル・単位・範囲・カレンダー）。
    pub meta: Meta,
    /// 宣言されたレーンの一覧（`order` フィールドで表示順を制御）。
    pub lanes: Vec<Lane>,
    /// 年表アイテムの一覧（Span / Event / EventRange の tagged union）。
    pub items: Vec<Item>,
    /// Wikidata インポートの記録（インポートなしの場合は空）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<ImportRecord>,
    /// アイテムの出典・ライセンス情報（出典なしの場合は空）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SourceRecord>,
}

/// 年表のメタデータ。`timeline` ブロックの属性に対応する。
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Meta {
    /// 年表の表示タイトル。
    pub title: String,
    /// 時間軸の単位（通常 `"year"`）。
    pub unit: String,
    /// 表示範囲 `(start, end)`（年に丸めた値。月日精度は下の `range_*_month/_day` で保持）。
    pub range: (i64, i64),
    /// `range` start の月精度（`range 1939-09..1945-09` のような場合のみ Some）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_start_month: Option<u8>,
    /// `range` start の日精度。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_start_day: Option<u8>,
    /// `range` start の時精度。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_start_hour: Option<u8>,
    /// `range` start の分精度。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_start_minute: Option<u8>,
    /// `range` end の月精度。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_end_month: Option<u8>,
    /// `range` end の日精度。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_end_day: Option<u8>,
    /// `range` end の時精度。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_end_hour: Option<u8>,
    /// `range` end の分精度。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_end_minute: Option<u8>,
    /// `range` start の秒精度（ADR 0003 D1）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_start_second: Option<u8>,
    /// `range` start の UTC オフセット（分単位。ADR 0003 D1/D2）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_start_offset_minutes: Option<i16>,
    /// `range` end の秒精度（ADR 0003 D1）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_end_second: Option<u8>,
    /// `range` end の UTC オフセット（分単位。ADR 0003 D1/D2）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_end_offset_minutes: Option<i16>,
    /// 使用するカレンダー体系（例: `"proleptic_gregorian"`）。
    pub calendar: String,
    /// タグ→CSS カラー文字列のマッピング（`color_map` ブロックで宣言）。
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub color_map: std::collections::HashMap<String, String>,
}

/// 年表のレーン（行）を表す。`lane` 宣言に対応する。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Lane {
    /// レーンの一意識別子（`as` 節または自動生成スラッグ）。
    pub id: String,
    /// 表示用ラベル文字列。
    pub label: String,
    /// レーンの種別（例: `"dynasty"`, `"custom"`）。
    pub kind: String,
    /// 表示順（小さいほど上に配置）。
    pub order: i64,
    /// 所属するグループ名（`group` ブロックで宣言された場合のみ Some）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// lane に明示指定された色（`color "#4a9eff";`）。未指定なら `None`。
    ///
    /// レンダラは lane の並び順からパレット色を機械的に割り当てるため、
    /// lane を 1 つ足したり `order` を変えると既存 lane の色が全部ずれる。
    /// これを固定する手段（#747）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// DSL ソース上の lane 宣言位置（Goto Definition 用）。ソーステキストを渡した場合のみ付与。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_span: Option<SourceSpan>,
}

/// 年表アイテムを表す tagged enum。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Item {
    /// 開始〜終了の期間を持つアイテム（王朝・時代など）。
    Span {
        id: String,
        lane: String,
        start: i64,
        end: i64,
        label: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        origin: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        link: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        // Precision fields (month/day/hour/minute)
        #[serde(skip_serializing_if = "Option::is_none")]
        start_month: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_day: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_hour: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_minute: Option<u8>,
        /// start の秒精度（ADR 0003 D1）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start_second: Option<u8>,
        /// start の UTC オフセット（分単位。`None` は「オフセットなしの裸の暦時刻」を意味する（ADR 0003 D2/D3）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start_offset_minutes: Option<i16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_month: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_day: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_hour: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_minute: Option<u8>,
        /// end の秒精度（ADR 0003 D1）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        end_second: Option<u8>,
        /// end の UTC オフセット（分単位。ADR 0003 D2/D3）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        end_offset_minutes: Option<i16>,
        /// `end` が `now`（継続中）で補完されたかどうか（#550）。`end` 自体は常に具体値（lowering 時の
        /// 現在年）を保持するが、`end_open == true` の場合 renderer / decompile はそれを
        /// 「継続中」として扱う。既存 IR との後方互換のため `false` の場合はシリアライズしない。
        #[serde(default, skip_serializing_if = "is_false")]
        end_open: bool,
        /// DSL ソース上の定義位置（双方向ジャンプ用）。
        #[serde(skip_serializing_if = "Option::is_none")]
        source_span: Option<SourceSpan>,
    },
    Event {
        id: String,
        lane: String,
        time: i64,
        label: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        origin: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        link: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        // Precision fields
        #[serde(skip_serializing_if = "Option::is_none")]
        time_month: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        time_day: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        time_hour: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        time_minute: Option<u8>,
        /// time の秒精度（ADR 0003 D1）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        time_second: Option<u8>,
        /// time の UTC オフセット（分単位。ADR 0003 D2/D3）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        time_offset_minutes: Option<i16>,
        /// DSL ソース上の定義位置（双方向ジャンプ用）。
        #[serde(skip_serializing_if = "Option::is_none")]
        source_span: Option<SourceSpan>,
    },
    EventRange {
        id: String,
        lane: String,
        start: i64,
        end: i64,
        label: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        origin: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        link: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        // Precision fields
        #[serde(skip_serializing_if = "Option::is_none")]
        start_month: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_day: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_hour: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_minute: Option<u8>,
        /// start の秒精度（ADR 0003 D1）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start_second: Option<u8>,
        /// start の UTC オフセット（分単位。ADR 0003 D2/D3）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start_offset_minutes: Option<i16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_month: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_day: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_hour: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_minute: Option<u8>,
        /// end の秒精度（ADR 0003 D1）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        end_second: Option<u8>,
        /// end の UTC オフセット（分単位。ADR 0003 D2/D3）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        end_offset_minutes: Option<i16>,
        /// `end` が `now`（継続中）で補完されたかどうか（#550）。Span のドキュメントと同様。
        #[serde(default, skip_serializing_if = "is_false")]
        end_open: bool,
        /// DSL ソース上の定義位置（双方向ジャンプ用）。
        #[serde(skip_serializing_if = "Option::is_none")]
        source_span: Option<SourceSpan>,
    },
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Wikidata インポートの記録。どのエンティティがどのアイテムにマップされたかを保持する。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImportRecord {
    /// インポート元のソース種別（例: `"wikidata"`）。
    pub source: String,
    /// Wikidata エンティティ ID（例: `"Q7209"`）。
    pub qid: String,
    /// マップ先のアイテム ID。
    pub mapped_to: String,
}

/// アイテムの出典・ライセンス情報。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceRecord {
    /// 出典の識別子（例: `"wd:Q7209"`）。
    pub id: String,
    /// プロバイダー名（例: `"wikidata"`）。
    pub provider: String,
    /// ライセンス識別子（例: `"CC0"`）。
    pub license: String,
}

// ─── 範囲補完ヘルパ（仕様 §1.4） ────────────────────────────────────

/// 範囲の **start** として `(year, month?, day?)` を fractional year に変換する。
///
/// month/day が `None` の場合は年/月の頭（1月1日 / 月の1日）を採用する。
/// Renderer の x 座標計算で使われる想定。
pub fn start_frac(year: i64, month: Option<u8>, day: Option<u8>) -> f64 {
    let m = month.unwrap_or(1).clamp(1, 12);
    let d = day.unwrap_or(1).clamp(1, days_in_month(year, m));
    year as f64 + (m - 1) as f64 / 12.0 + (d - 1) as f64 / 365.25
}

/// 範囲の **end** として `(year, month?, day?)` を fractional year に変換する。
///
/// month が `None` のときは年末（12月31日）、day が `None` のときは月末日を採用する。
pub fn end_frac(year: i64, month: Option<u8>, day: Option<u8>) -> f64 {
    let m = month.unwrap_or(12).clamp(1, 12);
    let last_day = days_in_month(year, m);
    let d = day.unwrap_or(last_day).clamp(1, last_day);
    year as f64 + (m - 1) as f64 / 12.0 + (d - 1) as f64 / 365.25
}

/// 指定 `(year, month)` の最終日（うるう年考慮）。`month` は 1..=12 を想定する。
pub fn days_in_month(year: i64, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// 時刻を分解して持つフィールド群（年 + 月日時分秒 + UTC オフセット）。
///
/// IR の `Item` は `start_month` / `start_day` / … のように時刻を平坦な
/// フィールドとして持つため、これを扱う関数は**同型の `Option<u8>` が
/// 5 連続する引数列**になりがちだった。取り違えてもコンパイラは検出できず、
/// `#[allow(clippy::too_many_arguments)]` を付けて回る状態になっていた
/// （`implementation-strict.md` §2-6 は `#[allow(clippy::*)]` の安易な追加を
/// NO-GO としている）。#772 で CLI 側に適用した「同型引数連続 → 名前付き
/// 構造体化」を、時刻分解引数にも広げたもの（#805）。
///
/// **IR のフィールドそのものは変えない。** 平坦なフィールドは JSON
/// スキーマの一部で、変えると後方互換が壊れる。ここは関数の引数を束ねる
/// ためだけの型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TimeParts {
    pub year: i64,
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    pub second: Option<u8>,
    /// UTC オフセット（分）。`None` はオフセット指定なし。
    pub offset_minutes: Option<i16>,
}

impl TimeParts {
    /// 年精度だけの時刻。
    pub fn from_year(year: i64) -> Self {
        Self {
            year,
            ..Default::default()
        }
    }
}
