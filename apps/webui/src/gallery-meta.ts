import apollo11Source from '../../../examples/apollo_11.tdsl?raw'
import apollo11HourlySource from '../../../examples/apollo_11_hourly.tdsl?raw'
import chinaDynastiesFilteredSource from '../../../examples/china_dynasties_filtered.tdsl?raw'
import chinaDynastiesSource from '../../../examples/china_dynasties.tdsl?raw'
import chinaWithImportSource from '../../../examples/china_with_import.tdsl?raw'
import featureShowcaseSource from '../../../examples/feature_showcase.tdsl?raw'
import fictionalEmpireSource from '../../../examples/fictional_empire.tdsl?raw'
import globalConferenceTimezonesSource from '../../../examples/global_conference_timezones.tdsl?raw'
import groupedDynastiesSource from '../../../examples/grouped_dynasties.tdsl?raw'
import internetHistorySource from '../../../examples/internet_history.tdsl?raw'
import issDockingSecondPrecisionSource from '../../../examples/iss_docking_second_precision.tdsl?raw'
import japaneseHistorySource from '../../../examples/japanese_history.tdsl?raw'
import officeholderWikidataSource from '../../../examples/officeholder_wikidata.tdsl?raw'
import samuraiWikidataSource from '../../../examples/samurai_wikidata.tdsl?raw'
import sciTechTimelineSource from '../../../examples/sci_tech_timeline.tdsl?raw'
import templateApplyExampleSource from '../../../examples/template_apply_example.tdsl?raw'
import worldWarsSource from '../../../examples/world_wars.tdsl?raw'

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
    description: 'span / event / event_range の基本文法と静的 item id を例示',
    filename: 'china_dynasties.tdsl',
    requiresNetwork: false,
    source: chinaDynastiesSource,
  },
  {
    label: '日本史年表（奈良〜江戸）',
    description: '複数 lane による時代区分・span/event/event_range と静的 item id を例示',
    filename: 'japanese_history.tdsl',
    requiresNetwork: false,
    source: japaneseHistorySource,
  },
  {
    label: '近代戦争年表',
    description: '多軸 lane と月日精度の event_range（1939-09-01..1945-09-02）を例示',
    filename: 'world_wars.tdsl',
    requiresNetwork: false,
    source: worldWarsSource,
  },
  {
    label: '科学技術の発明・発見年表',
    description: '発明・発見・通信・計算機の4 lane と多数の event item id を例示',
    filename: 'sci_tech_timeline.tdsl',
    requiresNetwork: false,
    source: sciTechTimelineSource,
  },
  {
    label: '架空帝国年表（フィクション）',
    description: '架空の王国・出来事、CSV（fictional_empire_items.csv）連携導線、color_map を例示',
    filename: 'fictional_empire.tdsl',
    requiresNetwork: false,
    source: fictionalEmpireSource,
  },
  {
    label: '中国王朝年表（グループ版）',
    description: 'group ブロックによる lane グルーピングとグループ外 lane の混在を例示',
    filename: 'grouped_dynasties.tdsl',
    requiresNetwork: false,
    source: groupedDynastiesSource,
  },
  {
    label: 'インターネット・Web 年表',
    description: '複数 lane（Web / プラットフォーム / SNS）と span/event 混在を例示',
    filename: 'internet_history.tdsl',
    requiresNetwork: false,
    source: internetHistorySource,
  },
  {
    label: 'Apollo 11 ミッション',
    description: '月日精度の日付（1969-07-16 など）と短期ミッション span を例示',
    filename: 'apollo_11.tdsl',
    requiresNetwork: false,
    source: apollo11Source,
  },
  {
    label: '中国王朝年表（Wikidata連携）',
    description: 'CLI専用・構文参考: import ブロックで Wikidata エンティティを取得し span にマッピング',
    filename: 'china_with_import.tdsl',
    requiresNetwork: true,
    source: chinaWithImportSource,
  },
  {
    label: '戦国武将生没年（Wikidata連携）',
    description: 'CLI専用・構文参考: P569/P570 で人物の生没年を span にマッピング',
    filename: 'samurai_wikidata.tdsl',
    requiresNetwork: true,
    source: samuraiWikidataSource,
  },
  {
    label: '中国王朝年表（SPARQL + filter）',
    description: 'CLI専用・構文参考: SPARQL query と filter 句で複数 entity import を例示',
    filename: 'china_dynasties_filtered.tdsl',
    requiresNetwork: true,
    source: chinaDynastiesFilteredSource,
  },
  {
    label: 'テンプレート構文（template / apply）',
    description: 'CLI専用・構文参考: template/apply と policy field_priority による再利用マッピングを例示',
    filename: 'template_apply_example.tdsl',
    requiresNetwork: true,
    source: templateApplyExampleSource,
  },
  {
    label: '公職在任期間（Wikidata qualifier）',
    description: 'CLI専用・構文参考: expand claim(P39) と qualifier(P580/P582) の event_range マッピングを例示',
    filename: 'officeholder_wikidata.tdsl',
    requiresNetwork: true,
    source: officeholderWikidataSource,
  },
  {
    label: '機能ショーケース（note / link / color / now）',
    description: 'block_options（note / link / color）と open-ended `now` の使用例を静的定義で例示',
    filename: 'feature_showcase.tdsl',
    requiresNetwork: false,
    source: featureShowcaseSource,
  },
  {
    label: 'アポロ11号 月面着陸日（時精度）',
    description: 'unit hour による日単位より細かい時刻軸目盛りを例示',
    filename: 'apollo_11_hourly.tdsl',
    requiresNetwork: false,
    source: apollo11HourlySource,
  },
  {
    label: 'ISS ドッキング（秒精度・UTCオフセット）',
    description: 'unit second と UTC(`Z`)オフセット付き時刻値による秒精度タイムラインを例示',
    filename: 'iss_docking_second_precision.tdsl',
    requiresNetwork: false,
    source: issDockingSecondPrecisionSource,
  },
  {
    label: '国際カンファレンス（複数タイムゾーン）',
    description: '複数タイムゾーン（+09:00 / -05:00 / Z）を使ったoffset付き値同士のUTC正規化比較を例示',
    filename: 'global_conference_timezones.tdsl',
    requiresNetwork: false,
    source: globalConferenceTimezonesSource,
  },
]
