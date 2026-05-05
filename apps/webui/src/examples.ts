export interface Example {
  label: string
  source: string
}

export const EXAMPLES: Example[] = [
  {
    label: '中国王朝（静的）',
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
]
