use crate::ir::TimeParts;
use crate::ir::{LaneKind, SourceSpan, TimelineIr, known_lane_kinds};
use tdsl_parser::ast;

/// `(year, month_or_0, day_or_0, hour_or_0, minute_or_0, second_or_0)` を返す。
/// 精度が `None` の場合はソート上は最小値扱い。offset は含まない
/// （offset付き/なしの比較は `compare_ir_time` で ADR 0003 D2 に従い明示的に扱う）。
fn sortable_tuple(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
    second: Option<u8>,
) -> (i64, u8, u8, u8, u8, u8) {
    (
        year,
        month.unwrap_or(0),
        day.unwrap_or(0),
        hour.unwrap_or(0),
        minute.unwrap_or(0),
        second.unwrap_or(0),
    )
}

/// UTC からのオフセット（分単位）を差し引いた civil time を UTC 秒相当の整数に正規化する。
/// `lower::days_from_civil` の civil-date-to-days 変換を再利用し（DRY）、時刻部分を加算する。
fn normalize_ir_time_utc(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
    second: Option<u8>,
    offset_minutes: i16,
) -> i128 {
    let days =
        crate::lower::days_from_civil(year, month.unwrap_or(1).max(1), day.unwrap_or(1).max(1));
    let seconds = i128::from(hour.unwrap_or(0)) * 3600
        + i128::from(minute.unwrap_or(0)) * 60
        + i128::from(second.unwrap_or(0))
        - i128::from(offset_minutes) * 60;
    days * 86_400 + seconds
}

/// ADR 0003 D2 に準拠した、IR プリミティブフィールドからの時刻比較。
///
/// - offset 付き同士は UTC相当に正規化して比較する。
/// - offset なし同士は、従来どおり暦時刻の値そのもので比較する。
/// - 片方のみ offset 付きの場合は曖昧な比較として `None` を返す
///   （`validate.rs` は警告のみを扱うため、lowering の `MixedOffsetComparison`
///   エラーとは異なり、呼び出し側が専用の警告メッセージを生成する）。
///
/// 以前は 14 引数（a_* / b_* の月日時分秒がそれぞれ同型で連続）を位置渡し
/// しており、取り違えてもコンパイラが検出できなかった。`TimeParts` にまとめ、
/// `#[allow(clippy::too_many_arguments)]` を撤去した（#805）。
fn compare_ir_time(a: TimeParts, b: TimeParts) -> Option<std::cmp::Ordering> {
    let (a_year, a_month, a_day, a_hour, a_minute, a_second, a_offset) = (
        a.year,
        a.month,
        a.day,
        a.hour,
        a.minute,
        a.second,
        a.offset_minutes,
    );
    let (b_year, b_month, b_day, b_hour, b_minute, b_second, b_offset) = (
        b.year,
        b.month,
        b.day,
        b.hour,
        b.minute,
        b.second,
        b.offset_minutes,
    );
    match (a_offset, b_offset) {
        (Some(off_a), Some(off_b)) => {
            let norm_a =
                normalize_ir_time_utc(a_year, a_month, a_day, a_hour, a_minute, a_second, off_a);
            let norm_b =
                normalize_ir_time_utc(b_year, b_month, b_day, b_hour, b_minute, b_second, off_b);
            Some(norm_a.cmp(&norm_b))
        }
        (None, None) => Some(
            sortable_tuple(a_year, a_month, a_day, a_hour, a_minute, a_second).cmp(
                &sortable_tuple(b_year, b_month, b_day, b_hour, b_minute, b_second),
            ),
        ),
        _ => None,
    }
}

/// 分解された時刻を人間可読な文字列にする。
fn format_time(t: TimeParts) -> String {
    let (year, month, day, hour, minute, second, offset_minutes) = (
        t.year,
        t.month,
        t.day,
        t.hour,
        t.minute,
        t.second,
        t.offset_minutes,
    );
    let base = match (month, day, hour, minute) {
        (Some(m), Some(d), Some(h), Some(min)) => {
            format!("{year:04}-{m:02}-{d:02}T{h:02}:{min:02}")
        }
        (Some(m), Some(d), _, _) => return format!("{year:04}-{m:02}-{d:02}"),
        (Some(m), _, _, _) => return format!("{year:04}-{m:02}"),
        _ => return year.to_string(),
    };
    let with_second = match second {
        Some(s) => format!("{base}:{s:02}"),
        None => base,
    };
    match offset_minutes {
        Some(off) => format!("{with_second}{}", crate::lower::format_offset_suffix(off)),
        None => with_second,
    }
}

/// AST（lowering 前）レベルで検出できる参照エラー。`span` はソース内のバイト範囲。
///
/// `map` / `apply` ブロックの参照のうち、**ネットワーク（Wikidata 取得）を要しない**
/// 静的に判定可能な参照ミスを表す。具体的には lowering Pass 4 が
/// `UnresolvedImport` / `UnknownTemplate` として検出するエラーのうち、
/// エンティティ解決に依存しない部分（import alias / template の宣言有無、
/// `map` 参照の `alias.key` 形式）を offline でも報告するために使う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceDiagnostic {
    /// エラーメッセージ。
    pub message: String,
    /// 該当する `map` / `apply` 文のソース内バイト範囲。
    pub span: ast::Span,
}

/// AST から、ネットワーク不要で判定できる `map` / `apply` の参照エラーを収集する。
///
/// - `map <alias>.<key>` の `alias` が `import ... as <alias>` で宣言されていない → エラー。
/// - `apply <template> to <import>` の `import` が未宣言 → エラー。
/// - `apply <template> to <import>` の `template` が未宣言 → エラー。
///
/// `map` の参照は文法（`dotted_ident`）上必ず `alias.key` 形式（`.` を含む）なので、
/// 形式不正はパース段階で弾かれる（ここには到達しない）。
/// エンティティキー（`alias.<key>` の `key`）が Wikidata に存在するかは
/// ネットワークに依存するため、ここでは判定しない（lowering Pass 4 の責務）。
/// alias / template の宣言解決は lowering Pass 1 / Pass 3 と同じ規則
/// （`import` は `alias ?? source_type`、`template` は `alias ?? name` をキーとする）。
pub fn validate_static_references(file: &ast::File) -> Vec<ReferenceDiagnostic> {
    let mut import_aliases: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut template_keys: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for stmt in &file.statements {
        match &stmt.node {
            ast::Statement::Import(imp) => {
                let alias = imp.alias.as_deref().unwrap_or(imp.source_type.as_str());
                import_aliases.insert(alias);
            }
            ast::Statement::Template(t) => {
                let key = t.alias.as_deref().unwrap_or(t.name.as_str());
                template_keys.insert(key);
            }
            _ => {}
        }
    }

    let mut diags = Vec::new();
    for stmt in &file.statements {
        match &stmt.node {
            ast::Statement::Map(m) => {
                // 文法上 source_ref は必ず `alias.key`（dot を含む）。先頭要素が import alias。
                let alias = m
                    .source_ref
                    .split('.')
                    .next()
                    .unwrap_or(m.source_ref.as_str());
                if !import_aliases.contains(alias) {
                    diags.push(ReferenceDiagnostic {
                        message: format!("Map references undeclared import alias: {alias}"),
                        span: stmt.span,
                    });
                }
            }
            ast::Statement::Apply(a) => {
                if !import_aliases.contains(a.import_alias.as_str()) {
                    diags.push(ReferenceDiagnostic {
                        message: format!(
                            "Apply references undeclared import alias: {}",
                            a.import_alias
                        ),
                        span: stmt.span,
                    });
                }
                if !template_keys.contains(a.template_alias.as_str()) {
                    diags.push(ReferenceDiagnostic {
                        message: format!(
                            "Apply references undeclared template: {}",
                            a.template_alias
                        ),
                        span: stmt.span,
                    });
                }
            }
            _ => {}
        }
    }
    diags
}

/// 診断メッセージと、対応するアイテムのソース位置（あれば）を保持する構造体。
///
/// LSP の診断（`publishDiagnostics`）や将来の構造化出力で使用する。
/// `span` が `None` の場合はドキュメント先頭などの妥当なデフォルト位置を使用する。
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationDiagnostic {
    /// `docs/error-catalog.md` に対応する安定した診断コード（`"W205"` 等）。
    ///
    /// CI で特定の警告だけを許容/禁止できるようにするための識別子（#748）。
    /// **カタログの見出し（`### W205: …`）と 1 対 1 で対応させること。**
    pub code: &'static str,
    /// 警告メッセージ（既存 `validate()` と同じ文字列）。
    ///
    /// **コードは混ぜない。** 混ぜると `validate()` の戻り値を使っている
    /// 既存の呼び出し元の出力が変わる。表示側がコードを添える。
    pub message: String,
    /// 対応するアイテムの `source_span`。アイテムに紐付かない警告は `None`。
    pub span: Option<SourceSpan>,
}

/// 構造化バリデーション診断を返す。`validate()` の上位互換。
///
/// 各警告を該当アイテムの `source_span` に紐付ける。`source_span` は
/// `lower_static_with_source` / `lower_with_wikidata_and_source` でソースを
/// 渡した場合のみ付与される（lowering 時点でソースなしなら常に `None`）。
pub fn validate_with_spans(ir: &TimelineIr) -> Vec<ValidationDiagnostic> {
    let mut diags = Vec::new();

    // Check lane kinds. Unknown explicit categories are warnings rather than
    // lowering errors; `custom` is the documented escape hatch.
    for lane in &ir.lanes {
        if LaneKind::parse(&lane.kind).is_none() {
            diags.push(ValidationDiagnostic {
                code: "W204",
                message: format!(
                    "Lane \"{}\" uses unknown kind: {} (known kinds: {}; use custom for user-defined categories)",
                    lane.id,
                    lane.kind,
                    known_lane_kinds()
                ),
                span: lane.source_span.clone(),
            });
        }
    }

    // Check that all item lanes exist
    let lane_ids: std::collections::HashSet<&str> =
        ir.lanes.iter().map(|l| l.id.as_str()).collect();

    for item in &ir.items {
        let (lane, span) = match item {
            crate::ir::Item::Span {
                lane, source_span, ..
            } => (lane.as_str(), source_span.clone()),
            crate::ir::Item::Event {
                lane, source_span, ..
            } => (lane.as_str(), source_span.clone()),
            crate::ir::Item::EventRange {
                lane, source_span, ..
            } => (lane.as_str(), source_span.clone()),
        };
        if !lane_ids.contains(lane) {
            diags.push(ValidationDiagnostic {
                code: "W201",
                message: format!("Item references unknown lane: {lane}"),
                span,
            });
        }
    }

    // Check start > end for span and event_range items（月日・秒・offset精度を考慮、ADR 0003 D2）
    for item in &ir.items {
        match item {
            crate::ir::Item::Span {
                id,
                start,
                end,
                start_month,
                start_day,
                start_hour,
                start_minute,
                start_second,
                start_offset_minutes,
                end_month,
                end_day,
                end_hour,
                end_minute,
                end_second,
                end_offset_minutes,
                source_span,
                ..
            } => {
                match compare_ir_time(
                    TimeParts {
                        year: *start,
                        month: *start_month,
                        day: *start_day,
                        hour: *start_hour,
                        minute: *start_minute,
                        second: *start_second,
                        offset_minutes: *start_offset_minutes,
                    },
                    TimeParts {
                        year: *end,
                        month: *end_month,
                        day: *end_day,
                        hour: *end_hour,
                        minute: *end_minute,
                        second: *end_second,
                        offset_minutes: *end_offset_minutes,
                    },
                ) {
                    Some(std::cmp::Ordering::Greater) => {
                        let start_text = format_time(TimeParts {
                            year: *start,
                            month: *start_month,
                            day: *start_day,
                            hour: *start_hour,
                            minute: *start_minute,
                            second: *start_second,
                            offset_minutes: *start_offset_minutes,
                        });
                        let end_text = format_time(TimeParts {
                            year: *end,
                            month: *end_month,
                            day: *end_day,
                            hour: *end_hour,
                            minute: *end_minute,
                            second: *end_second,
                            offset_minutes: *end_offset_minutes,
                        });
                        diags.push(ValidationDiagnostic {
                            code: "W202",
                            message: format!(
                                "Span \"{id}\" has start ({start_text}) > end ({end_text})"
                            ),
                            span: source_span.clone(),
                        });
                    }
                    Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal) => {}
                    None => {
                        diags.push(ValidationDiagnostic {
                            code: "W208",
                            message: format!(
                                "Span \"{id}\" mixes a UTC-offset time value with a value that has no offset; cannot determine start/end order (ADR 0003 D2, make both sides consistent)"
                            ),
                            span: source_span.clone(),
                        });
                    }
                }
            }
            crate::ir::Item::EventRange {
                id,
                start,
                end,
                start_month,
                start_day,
                start_hour,
                start_minute,
                start_second,
                start_offset_minutes,
                end_month,
                end_day,
                end_hour,
                end_minute,
                end_second,
                end_offset_minutes,
                source_span,
                ..
            } => {
                match compare_ir_time(
                    TimeParts {
                        year: *start,
                        month: *start_month,
                        day: *start_day,
                        hour: *start_hour,
                        minute: *start_minute,
                        second: *start_second,
                        offset_minutes: *start_offset_minutes,
                    },
                    TimeParts {
                        year: *end,
                        month: *end_month,
                        day: *end_day,
                        hour: *end_hour,
                        minute: *end_minute,
                        second: *end_second,
                        offset_minutes: *end_offset_minutes,
                    },
                ) {
                    Some(std::cmp::Ordering::Greater) => {
                        let start_text = format_time(TimeParts {
                            year: *start,
                            month: *start_month,
                            day: *start_day,
                            hour: *start_hour,
                            minute: *start_minute,
                            second: *start_second,
                            offset_minutes: *start_offset_minutes,
                        });
                        let end_text = format_time(TimeParts {
                            year: *end,
                            month: *end_month,
                            day: *end_day,
                            hour: *end_hour,
                            minute: *end_minute,
                            second: *end_second,
                            offset_minutes: *end_offset_minutes,
                        });
                        diags.push(ValidationDiagnostic {
                            code: "W202",
                            message: format!(
                                "EventRange \"{id}\" has start ({start_text}) > end ({end_text})"
                            ),
                            span: source_span.clone(),
                        });
                    }
                    Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal) => {}
                    None => {
                        diags.push(ValidationDiagnostic {
                            code: "W208",
                            message: format!(
                                "EventRange \"{id}\" mixes a UTC-offset time value with a value that has no offset; cannot determine start/end order (ADR 0003 D2, make both sides consistent)"
                            ),
                            span: source_span.clone(),
                        });
                    }
                }
            }
            crate::ir::Item::Event { .. } => {}
        }
    }

    // Check range coherence（月日・秒・offset精度を考慮、ADR 0003 D2）— アイテムに紐付かないため span: None
    let (range_start, range_end) = ir.meta.range;
    let range_coherence = compare_ir_time(
        TimeParts {
            year: range_start,
            month: ir.meta.range_start_month,
            day: ir.meta.range_start_day,
            hour: ir.meta.range_start_hour,
            minute: ir.meta.range_start_minute,
            second: ir.meta.range_start_second,
            offset_minutes: ir.meta.range_start_offset_minutes,
        },
        TimeParts {
            year: range_end,
            month: ir.meta.range_end_month,
            day: ir.meta.range_end_day,
            hour: ir.meta.range_end_hour,
            minute: ir.meta.range_end_minute,
            second: ir.meta.range_end_second,
            offset_minutes: ir.meta.range_end_offset_minutes,
        },
    );
    let range_start_text = format_time(TimeParts {
        year: range_start,
        month: ir.meta.range_start_month,
        day: ir.meta.range_start_day,
        hour: ir.meta.range_start_hour,
        minute: ir.meta.range_start_minute,
        second: ir.meta.range_start_second,
        offset_minutes: ir.meta.range_start_offset_minutes,
    });
    let range_end_text = format_time(TimeParts {
        year: range_end,
        month: ir.meta.range_end_month,
        day: ir.meta.range_end_day,
        hour: ir.meta.range_end_hour,
        minute: ir.meta.range_end_minute,
        second: ir.meta.range_end_second,
        offset_minutes: ir.meta.range_end_offset_minutes,
    });
    match range_coherence {
        Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal) => {
            diags.push(ValidationDiagnostic {
                code: "W203",
                message: format!("Timeline range is invalid: {range_start_text}..{range_end_text}"),
                span: None,
            });
            // Range itself is incoherent; skip per-item containment checks below
            // (they would be meaningless against an invalid range).
            return diags;
        }
        Some(std::cmp::Ordering::Less) => {}
        None => {
            diags.push(ValidationDiagnostic {
                code: "W209",
                message: "Timeline range mixes a UTC-offset time value with a value that has no offset; cannot determine range coherence (ADR 0003 D2, make both sides consistent)".to_string(),
                span: None,
            });
            return diags;
        }
    }

    // #553: items entirely or partially outside `timeline.range` are
    // silently dropped (Event) or rendered off-canvas (Span/EventRange)
    // by the renderer. Warn here so authors can trace "written but not
    // shown" issues instead of hitting a silent fallback.
    for item in &ir.items {
        match item {
            crate::ir::Item::Event {
                id,
                time,
                time_month,
                time_day,
                time_hour,
                time_minute,
                time_second,
                time_offset_minutes,
                source_span,
                ..
            } => {
                let after_start = compare_ir_time(
                    TimeParts {
                        year: *time,
                        month: *time_month,
                        day: *time_day,
                        hour: *time_hour,
                        minute: *time_minute,
                        second: *time_second,
                        offset_minutes: *time_offset_minutes,
                    },
                    TimeParts {
                        year: range_start,
                        month: ir.meta.range_start_month,
                        day: ir.meta.range_start_day,
                        hour: ir.meta.range_start_hour,
                        minute: ir.meta.range_start_minute,
                        second: ir.meta.range_start_second,
                        offset_minutes: ir.meta.range_start_offset_minutes,
                    },
                );
                let before_end = compare_ir_time(
                    TimeParts {
                        year: *time,
                        month: *time_month,
                        day: *time_day,
                        hour: *time_hour,
                        minute: *time_minute,
                        second: *time_second,
                        offset_minutes: *time_offset_minutes,
                    },
                    TimeParts {
                        year: range_end,
                        month: ir.meta.range_end_month,
                        day: ir.meta.range_end_day,
                        hour: ir.meta.range_end_hour,
                        minute: ir.meta.range_end_minute,
                        second: ir.meta.range_end_second,
                        offset_minutes: ir.meta.range_end_offset_minutes,
                    },
                );
                match (after_start, before_end) {
                    (Some(a), Some(b)) => {
                        if a == std::cmp::Ordering::Less || b == std::cmp::Ordering::Greater {
                            let time_text = format_time(TimeParts {
                                year: *time,
                                month: *time_month,
                                day: *time_day,
                                hour: *time_hour,
                                minute: *time_minute,
                                second: *time_second,
                                offset_minutes: *time_offset_minutes,
                            });
                            diags.push(ValidationDiagnostic {
                                code: "W205",
                                message: format!(
                                    "Event \"{id}\" at {time_text} is outside timeline.range and will not be rendered"
                                ),
                                span: source_span.clone(),
                            });
                        }
                    }
                    _ => {
                        diags.push(ValidationDiagnostic {
                            code: "W208",
                            message: format!(
                                "Event \"{id}\" mixes a UTC-offset time value with a value that has no offset when compared against timeline.range; cannot determine containment (ADR 0003 D2, make both sides consistent)"
                            ),
                            span: source_span.clone(),
                        });
                    }
                }
            }
            crate::ir::Item::Span {
                id,
                start,
                end,
                start_month,
                start_day,
                start_hour,
                start_minute,
                start_second,
                start_offset_minutes,
                end_month,
                end_day,
                end_hour,
                end_minute,
                end_second,
                end_offset_minutes,
                source_span,
                ..
            }
            | crate::ir::Item::EventRange {
                id,
                start,
                end,
                start_month,
                start_day,
                start_hour,
                start_minute,
                start_second,
                start_offset_minutes,
                end_month,
                end_day,
                end_hour,
                end_minute,
                end_second,
                end_offset_minutes,
                source_span,
                ..
            } => {
                let kind = if matches!(item, crate::ir::Item::Span { .. }) {
                    "Span"
                } else {
                    "EventRange"
                };
                let start_end_order = compare_ir_time(
                    TimeParts {
                        year: *start,
                        month: *start_month,
                        day: *start_day,
                        hour: *start_hour,
                        minute: *start_minute,
                        second: *start_second,
                        offset_minutes: *start_offset_minutes,
                    },
                    TimeParts {
                        year: *end,
                        month: *end_month,
                        day: *end_day,
                        hour: *end_hour,
                        minute: *end_minute,
                        second: *end_second,
                        offset_minutes: *end_offset_minutes,
                    },
                );
                if start_end_order.is_none() {
                    // Already reported (as a mixed-offset warning) by the start > end check above.
                    continue;
                }
                if start_end_order == Some(std::cmp::Ordering::Greater) {
                    // Already reported by the start > end check above.
                    continue;
                }
                let end_after_range_start = compare_ir_time(
                    TimeParts {
                        year: *end,
                        month: *end_month,
                        day: *end_day,
                        hour: *end_hour,
                        minute: *end_minute,
                        second: *end_second,
                        offset_minutes: *end_offset_minutes,
                    },
                    TimeParts {
                        year: range_start,
                        month: ir.meta.range_start_month,
                        day: ir.meta.range_start_day,
                        hour: ir.meta.range_start_hour,
                        minute: ir.meta.range_start_minute,
                        second: ir.meta.range_start_second,
                        offset_minutes: ir.meta.range_start_offset_minutes,
                    },
                );
                let start_before_range_end = compare_ir_time(
                    TimeParts {
                        year: *start,
                        month: *start_month,
                        day: *start_day,
                        hour: *start_hour,
                        minute: *start_minute,
                        second: *start_second,
                        offset_minutes: *start_offset_minutes,
                    },
                    TimeParts {
                        year: range_end,
                        month: ir.meta.range_end_month,
                        day: ir.meta.range_end_day,
                        hour: ir.meta.range_end_hour,
                        minute: ir.meta.range_end_minute,
                        second: ir.meta.range_end_second,
                        offset_minutes: ir.meta.range_end_offset_minutes,
                    },
                );
                let start_after_range_start = compare_ir_time(
                    TimeParts {
                        year: *start,
                        month: *start_month,
                        day: *start_day,
                        hour: *start_hour,
                        minute: *start_minute,
                        second: *start_second,
                        offset_minutes: *start_offset_minutes,
                    },
                    TimeParts {
                        year: range_start,
                        month: ir.meta.range_start_month,
                        day: ir.meta.range_start_day,
                        hour: ir.meta.range_start_hour,
                        minute: ir.meta.range_start_minute,
                        second: ir.meta.range_start_second,
                        offset_minutes: ir.meta.range_start_offset_minutes,
                    },
                );
                let end_before_range_end = compare_ir_time(
                    TimeParts {
                        year: *end,
                        month: *end_month,
                        day: *end_day,
                        hour: *end_hour,
                        minute: *end_minute,
                        second: *end_second,
                        offset_minutes: *end_offset_minutes,
                    },
                    TimeParts {
                        year: range_end,
                        month: ir.meta.range_end_month,
                        day: ir.meta.range_end_day,
                        hour: ir.meta.range_end_hour,
                        minute: ir.meta.range_end_minute,
                        second: ir.meta.range_end_second,
                        offset_minutes: ir.meta.range_end_offset_minutes,
                    },
                );
                match (
                    end_after_range_start,
                    start_before_range_end,
                    start_after_range_start,
                    end_before_range_end,
                ) {
                    (Some(e_vs_rs), Some(s_vs_re), Some(s_vs_rs), Some(e_vs_re)) => {
                        let entirely_outside = e_vs_rs == std::cmp::Ordering::Less
                            || s_vs_re == std::cmp::Ordering::Greater;
                        let partially_outside = s_vs_rs == std::cmp::Ordering::Less
                            || e_vs_re == std::cmp::Ordering::Greater;
                        if entirely_outside {
                            diags.push(ValidationDiagnostic {
                                code: "W206",
                                message: format!(
                                    "{kind} \"{id}\" is entirely outside timeline.range and will not be rendered"
                                ),
                                span: source_span.clone(),
                            });
                        } else if partially_outside {
                            diags.push(ValidationDiagnostic {
                                code: "W207",
                                message: format!(
                                    "{kind} \"{id}\" is partially outside timeline.range and will be clipped"
                                ),
                                span: source_span.clone(),
                            });
                        }
                    }
                    _ => {
                        diags.push(ValidationDiagnostic {
                            code: "W208",
                            message: format!(
                                "{kind} \"{id}\" mixes a UTC-offset time value with a value that has no offset when compared against timeline.range; cannot determine containment (ADR 0003 D2, make both sides consistent)"
                            ),
                            span: source_span.clone(),
                        });
                    }
                }
            }
        }
    }

    diags
}

/// Validate the IR for semantic consistency.
///
/// `validate_with_spans` の薄いラッパ。既存の呼び出し元との後方互換を保つ。
/// 出力文字列は完全に現状維持する。
pub fn validate(ir: &TimelineIr) -> Vec<String> {
    validate_with_spans(ir)
        .into_iter()
        .map(|d| d.message)
        .collect()
}
