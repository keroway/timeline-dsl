export interface Example {
  label: string
  source: string
}

export const EXAMPLES: Example[] = [
  {
    label: '中国王朝',
    source: `timeline "中国王朝年表" {
    title "中国王朝年表";
    unit year;
    range -500..600;
    calendar proleptic_gregorian;
}

lane "秦" as qin { kind dynasty; order 10; }
lane "漢" as han { kind dynasty; order 20; }
lane "三国時代" as sanguo { kind dynasty; order 30; }

span qin -221..-206 "秦" { tags ["dynasty"]; source wd:Q7183; };
span han -206..220 "漢" { tags ["dynasty"]; source wd:Q7209; };
span sanguo 220..280 "三国時代" { tags ["dynasty"]; };

event han -221 "秦の天下統一" {};
event han -202 "垓下の戦い" {};
event_range sanguo 228..234 "諸葛亮の北伐" { tags ["war"]; };
`,
  },
  {
    label: '近現代史',
    source: `timeline "近現代史" {
    title "近現代史";
    unit year;
    range 1800..2000;
    calendar proleptic_gregorian;
}

lane "世界" as world { kind era; order 10; }
lane "戦争" as wars { kind war; order 20; }

event_range world 1760..1840 "産業革命" { tags ["era"]; };
event_range wars 1914..1918 "第一次世界大戦" { tags ["war"]; };
event_range wars 1939..1945 "第二次世界大戦" { tags ["war"]; };
event_range world 1947..1991 "冷戦" { tags ["era"]; };

event world 1905 "アインシュタイン 相対性理論" {};
event world 1969 "アポロ11号 月面着陸" {};
`,
  },
  {
    label: 'DSL 基本文法（最小構成）',
    source: `// span / event / event_range の最小構成サンプル

timeline "DSL 基本文法" {
    title "span・event・event_range の使い方";
    unit year;
    range 0..100;
    calendar proleptic_gregorian;
}

// lane: イベントを分類するレーン
lane "王朝" as dynasty { kind dynasty; order 10; }
lane "出来事" as events { kind event;   order 20; }

// span: 期間を帯グラフで表示
span dynasty 0..50 "前半王朝" { tags ["dynasty"]; };
span dynasty 50..100 "後半王朝" { tags ["dynasty"]; };

// event: 特定年の点イベント
event events 30 "改革令" {};
event events 75 "転換点" { tags ["milestone"]; };

// event_range: 期間付き点イベント（塗りつぶし矩形）
event_range events 40..45 "内乱" { tags ["war"]; };
`,
  },
  {
    label: 'Wikidata インポート（オフライン不可）',
    source: `// ⚠️ このサンプルは Wikidata API を使用します。
// WebUI はオフラインモードで動作するため、
// "import wikidata" はコンパイルエラーになります。
// 構文の参考として確認してください。

timeline "Wikidata インポート例" {
    title "Wikidata から王朝データをインポート";
    unit year;
    range -300..300;
    calendar proleptic_gregorian;
}

lane "秦" as qin { kind dynasty; order 10; }
lane "漢" as han { kind dynasty; order 20; }

// Wikidata からエンティティをインポート
import wikidata as wd {
    entity Q7183 as qin_dynasty;  // 秦
    entity Q7209 as han_dynasty;  // 漢
    policy merge_by_source;
}

// インポートデータを span にマッピング
map wd.qin_dynasty to span {
    lane qin;
    start claim(P571).year;  // 成立年
    end   claim(P576).year;  // 消滅年
    label label@ja ?? label@en;
    tags ["dynasty", "imported"];
}

map wd.han_dynasty to span {
    lane han;
    start claim(P571).year;
    end   claim(P576).year;
    label label@ja ?? label@en;
    tags ["dynasty", "imported"];
}
`,
  },
]
