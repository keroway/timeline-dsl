use serde::{Deserialize, Serialize};

/// `.tdsl` ファイルをコンパイルした結果の正規中間表現（JSON 直列化対象）。
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    /// 年表の表示タイトル。
    pub title: String,
    /// 時間軸の単位（通常 `"year"`）。
    pub unit: String,
    /// 表示範囲 `(start, end)`（単位は `unit` に準ずる）。
    pub range: (i64, i64),
    /// 使用するカレンダー体系（例: `"proleptic_gregorian"`）。
    pub calendar: String,
    /// タグ→CSS カラー文字列のマッピング（`color_map` ブロックで宣言）。
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub color_map: std::collections::HashMap<String, String>,
}

/// 年表のレーン（行）を表す。`lane` 宣言に対応する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lane {
    /// レーンの一意識別子（`as` 節または自動生成スラッグ）。
    pub id: String,
    /// 表示用ラベル文字列。
    pub label: String,
    /// レーンの種別（例: `"dynasty"`, `"custom"`）。
    pub kind: String,
    /// 表示順（小さいほど上に配置）。
    pub order: i64,
}

/// 年表アイテムを表す tagged enum。
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    },
}

/// Wikidata インポートの記録。どのエンティティがどのアイテムにマップされたかを保持する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRecord {
    /// インポート元のソース種別（例: `"wikidata"`）。
    pub source: String,
    /// Wikidata エンティティ ID（例: `"Q7209"`）。
    pub qid: String,
    /// マップ先のアイテム ID。
    pub mapped_to: String,
}

/// アイテムの出典・ライセンス情報。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    /// 出典の識別子（例: `"wd:Q7209"`）。
    pub id: String,
    /// プロバイダー名（例: `"wikidata"`）。
    pub provider: String,
    /// ライセンス識別子（例: `"CC0"`）。
    pub license: String,
}
