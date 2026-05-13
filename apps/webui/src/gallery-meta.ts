export interface GalleryExample {
  label: string
  description: string
  filename: string
  requiresNetwork: boolean
  source: string
}

export const GALLERY_EXAMPLES: GalleryExample[] = [
  {
    label: '中国王朝年表',
    description: 'span / event / event_range の基本文法（紀元前500〜西暦2000年）',
    filename: 'china_dynasties.tdsl',
    requiresNetwork: false,
    source: `// 中国王朝年表サンプル

timeline "中国王朝年表" {
    title "中国王朝年表";
    unit year;
    range -500..2000;
    calendar proleptic_gregorian;
}

lane "秦" as qin { kind dynasty; order 10; }
lane "漢" as han { kind dynasty; order 20; }
lane "三国" as sanguo { kind dynasty; order 30; }

// 王朝の存続期間
span qin -221..-206 "秦" { tags ["dynasty"]; source wd:Q7183; id "span:qin"; };
span han -206..220 "漢" { tags ["dynasty"]; source wd:Q7209; id "span:han"; };
span sanguo 220..280 "三国時代" { tags ["dynasty"]; id "span:sanguo"; };

// 主要な出来事
event qin -221 "秦の天下統一" {};
event han -209 "陳勝・呉広の乱" {};
event han -202 "垓下の戦い" {};

// 期間イベント
event_range han 184..204 "黄巾の乱" { tags ["war"]; };
event_range sanguo 228..234 "諸葛亮の北伐" { tags ["war"]; };
`,
  },
  {
    label: '日本史年表（奈良〜江戸）',
    description: '複数 lane による時代区分・span/event/event_range の組み合わせ',
    filename: 'japanese_history.tdsl',
    requiresNetwork: false,
    source: `// 日本史年表サンプル（静的定義のみ）

timeline "日本史年表（奈良〜江戸）" {
    title "日本史年表（奈良〜江戸）";
    unit year;
    range 700..1900;
    calendar proleptic_gregorian;
}

lane "時代区分" as jidai { kind dynasty; order 10; }
lane "政治・戦乱" as seiji { kind event; order 20; }

span jidai 710..794   "奈良時代"       { tags ["era"]; id "span:nara"; };
span jidai 794..1185  "平安時代"       { tags ["era"]; id "span:heian"; };
span jidai 1185..1333 "鎌倉時代"       { tags ["era"]; id "span:kamakura"; };
span jidai 1336..1573 "室町時代"       { tags ["era"]; id "span:muromachi"; };
span jidai 1573..1603 "安土桃山時代"   { tags ["era"]; id "span:azuchi_momoyama"; };
span jidai 1603..1868 "江戸時代"       { tags ["era"]; id "span:edo"; };

event jidai 710   "平城京遷都" {};
event jidai 794   "平安京遷都" {};
event jidai 1192  "鎌倉幕府成立" {};
event jidai 1338  "室町幕府成立" {};
event jidai 1603  "江戸幕府成立" {};
event jidai 1868  "明治維新" {};

event_range seiji 1156..1160 "保元・平治の乱" { tags ["war"]; };
event_range seiji 1180..1185 "源平合戦"       { tags ["war"]; };
event_range seiji 1274..1281 "元寇"           { tags ["war"]; };
event_range seiji 1467..1477 "応仁の乱"       { tags ["war"]; };
event_range seiji 1560..1600 "戦国〜天下統一" { tags ["war"]; };
`,
  },
  {
    label: '近代戦争年表',
    description: '多軸 lane と event_range で第一次・第二次世界大戦を表現',
    filename: 'world_wars.tdsl',
    requiresNetwork: false,
    source: `// 近代戦争年表サンプル（静的定義のみ）

timeline "近代戦争年表" {
    title "近代戦争年表（第一次・第二次世界大戦）";
    unit year;
    range 1900..1950;
    calendar proleptic_gregorian;
}

lane "戦争" as war       { kind event; order 10; }
lane "政治" as politics  { kind event; order 20; }
lane "社会" as society   { kind event; order 30; }

event_range war 1914..1918 "第一次世界大戦" { tags ["war", "wwi"]; id "range:wwi"; };
event war     1914 "サラエボ事件（開戦の引き金）" { tags ["wwi"]; };
event politics 1917 "ロシア革命"                  { tags ["wwi", "revolution"]; };
event war     1918 "第一次世界大戦終結"            { tags ["wwi"]; };
event politics 1919 "パリ講和会議・ヴェルサイユ条約" { tags ["wwi", "treaty"]; };

event politics 1929 "世界恐慌"       { tags ["economy"]; };
event politics 1933 "ナチス政権樹立" { tags ["politics"]; };

event_range war 1939..1945 "第二次世界大戦" { tags ["war", "wwii"]; id "range:wwii"; };
event_range war 1941..1945 "太平洋戦争"     { tags ["war", "wwii", "pacific"]; id "range:pacific_war"; };

event war     1939 "ドイツ、ポーランド侵攻（WWII開戦）" { tags ["wwii"]; };
event war     1940 "ダンケルク撤退・フランス陥落"         { tags ["wwii"]; };
event war     1941 "独ソ戦開始・日本の真珠湾攻撃"         { tags ["wwii"]; };
event war     1945 "第二次世界大戦終結"                    { tags ["wwii"]; };
event society 1945 "国際連合設立"                          { tags ["wwii", "international"]; };
event society 1918 "スペイン風邪（パンデミック）" { tags ["pandemic", "wwi"]; };
event society 1920 "国際連盟発足"               { tags ["international"]; };
`,
  },
  {
    label: '科学技術の発明・発見年表',
    description: '発明・発見・通信・計算機の4 lane で技術史を point イベント表現',
    filename: 'sci_tech_timeline.tdsl',
    requiresNetwork: false,
    source: `// 科学技術年表サンプル（静的定義のみ）

timeline "科学技術の発明・発見年表" {
    title "科学技術の発明・発見年表";
    unit year;
    range 1400..2000;
    calendar proleptic_gregorian;
}

lane "発明"   as invention   { kind event; order 10; }
lane "発見"   as discovery   { kind event; order 20; }
lane "通信"   as communication { kind event; order 30; }
lane "計算機" as computing   { kind event; order 40; }

event invention 1450 "グーテンベルクの活版印刷術"     { tags ["mechanical", "printing"]; };
event invention 1769 "ワットの蒸気機関改良"           { tags ["mechanical", "industrial"]; };
event invention 1885 "ガソリン自動車（ベンツ）"       { tags ["mechanical", "transport"]; };
event invention 1903 "ライト兄弟の動力飛行"           { tags ["mechanical", "transport"]; };

event discovery 1820 "電磁気の発見（エルステッド）"   { tags ["electrical"]; };
event invention 1879 "白熱電球（エジソン）"           { tags ["electrical"]; };
event discovery 1895 "X線の発見（レントゲン）"        { tags ["physics"]; };

event communication 1876 "電話の発明（ベル）"          { tags ["telephone"]; };
event communication 1895 "無線通信の発明（マルコーニ）" { tags ["radio"]; };
event communication 1969 "ARPANET（インターネットの原型）" { tags ["internet"]; };
event communication 1991 "World Wide Web 公開（バーナーズ＝リー）" { tags ["internet", "web"]; };

event computing 1936 "チューリングマシン（理論）"       { tags ["theory"]; };
event computing 1945 "ENIAC（世界初の汎用電子計算機）"  { tags ["hardware"]; };
event computing 1981 "IBM PC 発売"                      { tags ["hardware", "personal"]; };

event discovery 1859 "進化論（ダーウィン『種の起源』）"  { tags ["biology"]; };
event discovery 1953 "DNA二重らせん構造の解明"           { tags ["biology", "genetics"]; };
`,
  },
  {
    label: '架空帝国年表（フィクション）',
    description: '架空の王国・出来事を自由に定義するフィクション向けサンプル',
    filename: 'fictional_empire.tdsl',
    requiresNetwork: false,
    source: `timeline "ルメリア帝国年表" {
    title "ルメリア帝国年表";
    unit year;
    range 1000..1300;
    calendar proleptic_gregorian;
}

lane "王国" as kingdom { kind custom; order 10; }
lane "事件" as incidents { kind custom; order 20; }

span kingdom 1001..1180 "アルカディア王国" { tags ["dynasty", "fictional"]; id "span:arcadia"; };
event incidents 1042 "竜騎士団の創設" { tags ["founding", "fictional"]; id "event:knights"; };
event_range incidents 1175..1180 "黒霧戦争" { tags ["war", "fictional"]; id "range:black_mist"; };
event incidents 1201 "王都再建宣言" { tags ["reform", "fictional"]; id "event:rebuild"; };
`,
  },
  {
    label: '中国王朝年表（Wikidata連携）',
    description: 'import ブロックで Wikidata エンティティを取得・span にマッピング ⚠️ ネットワーク必要',
    filename: 'china_with_import.tdsl',
    requiresNetwork: true,
    source: `// 中国王朝年表 — Wikidataインポート付き
// ⚠️ WebUI は Wikidata API に接続できないため import はスキップされます。
// CLI でのオンラインビルド用途のサンプルとして参照してください。

timeline "中国王朝年表（Wikidata連携）" {
    title "中国王朝年表";
    unit year;
    range -500..300;
    calendar proleptic_gregorian;
}

lane "秦" as qin { kind dynasty; order 10; }
lane "漢" as han { kind dynasty; order 20; }

event qin -221 "秦の天下統一" {};
event han -209 "陳勝・呉広の乱" {};

import wikidata as wd {
    entity Q7183 as qin_dynasty;
    entity Q7209 as han_dynasty;
    policy merge_by_source;
}

map wd.qin_dynasty to span {
    lane qin;
    start claim(P571).year;
    end claim(P576).year;
    label label@ja ?? label@en;
    tags ["dynasty", "imported"];
}

map wd.han_dynasty to span {
    lane han;
    start claim(P571).year;
    end claim(P576).year;
    label label@ja ?? label@en;
    tags ["dynasty", "imported"];
}
`,
  },
  {
    label: '戦国武将生没年（Wikidata連携）',
    description: 'P569/P570 で人物の生没年を span にマッピング ⚠️ ネットワーク必要',
    filename: 'samurai_wikidata.tdsl',
    requiresNetwork: true,
    source: `// 戦国武将生没年サンプル（Wikidata連携）
// ⚠️ WebUI は Wikidata API に接続できないため import はスキップされます。
// CLI でのオンラインビルド用途のサンプルとして参照してください。

timeline "戦国武将生没年（Wikidata連携）" {
    title "戦国武将生没年";
    unit year;
    range 1520..1620;
    calendar proleptic_gregorian;
}

lane "織田信長" as nobunaga { kind person; order 10; }
lane "豊臣秀吉" as hideyoshi { kind person; order 20; }
lane "徳川家康" as ieyasu    { kind person; order 30; }

import wikidata as wd {
    entity Q193538 as oda_nobunaga;
    entity Q190497 as toyotomi_hideyoshi;
    entity Q45975  as tokugawa_ieyasu;
    policy merge_by_source;
}

map wd.oda_nobunaga to span {
    lane nobunaga;
    start claim(P569).year;
    end claim(P570).year;
    label label@ja ?? label@en;
    tags ["person", "sengoku"];
}

map wd.toyotomi_hideyoshi to span {
    lane hideyoshi;
    start claim(P569).year;
    end claim(P570).year;
    label label@ja ?? label@en;
    tags ["person", "sengoku"];
}

map wd.tokugawa_ieyasu to span {
    lane ieyasu;
    start claim(P569).year;
    end claim(P570).year;
    label label@ja ?? label@en;
    tags ["person", "sengoku"];
}
`,
  },
  {
    label: '中国王朝年表（SPARQL + filter）',
    description: 'SPARQL クエリで複数王朝を一括取得し filter 句で絞り込み ⚠️ ネットワーク必要',
    filename: 'china_dynasties_filtered.tdsl',
    requiresNetwork: true,
    source: `// 中国王朝年表 — filter 句で開始年が紀元前 300 年以降の王朝のみ抽出
// ⚠️ WebUI は Wikidata API に接続できないため import はスキップされます。
// CLI でのオンラインビルド用途のサンプルとして参照してください。

timeline "中国王朝年表（紀元前300年以降）" {
    title "中国王朝年表（紀元前300年以降）";
    unit year;
    range -300..1950;
    calendar proleptic_gregorian;
}

lane "中国王朝" as dynasty { kind dynasty; order 10; }

import wikidata as wd {
    query "SELECT ?item WHERE { ?item wdt:P31 wd:Q28171280 . }" as chinese_dynasties;
    policy merge_by_source;
}

map wd.chinese_dynasties to span {
    lane dynasty;
    filter claim(P571).year >= -300;
    filter claim(P576).year != null;
    start claim(P571).year;
    end   claim(P576).year;
    label label@ja ?? label@en;
    tags ["dynasty", "filtered"];
}
`,
  },
  {
    label: 'テンプレート構文（template / apply）',
    description: 'template で再利用可能なマップパターンを定義し apply で複数 import に適用 ⚠️ ネットワーク必要',
    filename: 'template_apply_example.tdsl',
    requiresNetwork: true,
    source: `// template / apply 構文のサンプル
// ⚠️ WebUI は Wikidata API に接続できないため import はスキップされます。
// CLI でのオンラインビルド用途のサンプルとして参照してください。

timeline "中国王朝年表（テンプレート版）" {
    title "中国王朝年表";
    unit year;
    range -500..700;
    calendar proleptic_gregorian;
}

lane "王朝" as dynasty { kind dynasty; order 1; }
lane "人物" as person { kind person; order 2; }

template "王朝スパン" as dynasty_span
    to span {
        start claim(P571).year;
        end claim(P576).year;
        label label@ja ?? label@en;
    }

template "人物の生涯" as person_life
    to event_range {
        start claim(P569).year;
        end claim(P570).year;
        label label@ja ?? label@en;
    }

import wikidata as dynasties {
    entity Q7209 as han;
    entity Q7183 as qin;
}

import wikidata as emperors {
    entity Q33857 as kangxi;
}

apply dynasty_span to dynasties {
    lane dynasty;
}

apply person_life to emperors {
    lane person;
}
`,
  },
]
