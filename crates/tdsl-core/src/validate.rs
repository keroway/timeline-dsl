use crate::ir::{SourceSpan, TimelineIr};
use tdsl_parser::ast;

/// `(year, month_or_0, day_or_0)` を返す。月日が `None` の場合はソート上は最小値扱い。
fn sortable_tuple(year: i64, month: Option<u8>, day: Option<u8>) -> (i64, u8, u8) {
    (year, month.unwrap_or(0), day.unwrap_or(0))
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
                        span: stmt.span.clone(),
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
                        span: stmt.span.clone(),
                    });
                }
                if !template_keys.contains(a.template_alias.as_str()) {
                    diags.push(ReferenceDiagnostic {
                        message: format!(
                            "Apply references undeclared template: {}",
                            a.template_alias
                        ),
                        span: stmt.span.clone(),
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
    /// 警告メッセージ（既存 `validate()` と同じ文字列）。
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
                message: format!("Item references unknown lane: {lane}"),
                span,
            });
        }
    }

    // Check start > end for span and event_range items（月日精度を考慮）
    for item in &ir.items {
        match item {
            crate::ir::Item::Span {
                id,
                start,
                end,
                start_month,
                start_day,
                end_month,
                end_day,
                source_span,
                ..
            } => {
                let s = sortable_tuple(*start, *start_month, *start_day);
                let e = sortable_tuple(*end, *end_month, *end_day);
                if s > e {
                    diags.push(ValidationDiagnostic {
                        message: format!("Span \"{id}\" has start ({start}) > end ({end})"),
                        span: source_span.clone(),
                    });
                }
            }
            crate::ir::Item::EventRange {
                id,
                start,
                end,
                start_month,
                start_day,
                end_month,
                end_day,
                source_span,
                ..
            } => {
                let s = sortable_tuple(*start, *start_month, *start_day);
                let e = sortable_tuple(*end, *end_month, *end_day);
                if s > e {
                    diags.push(ValidationDiagnostic {
                        message: format!("EventRange \"{id}\" has start ({start}) > end ({end})"),
                        span: source_span.clone(),
                    });
                }
            }
            crate::ir::Item::Event { .. } => {}
        }
    }

    // Check range coherence（月日精度を考慮）— アイテムに紐付かないため span: None
    let (range_start, range_end) = ir.meta.range;
    let r_start = sortable_tuple(
        range_start,
        ir.meta.range_start_month,
        ir.meta.range_start_day,
    );
    let r_end = sortable_tuple(range_end, ir.meta.range_end_month, ir.meta.range_end_day);
    if r_start >= r_end {
        diags.push(ValidationDiagnostic {
            message: format!("Timeline range is invalid: {range_start}..{range_end}"),
            span: None,
        });
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
