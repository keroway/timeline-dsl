use std::path::PathBuf;
use std::process;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

mod commands;

#[derive(Parser)]
#[command(name = "tdsl", version, about = "Timeline DSL compiler")]
struct Cli {
    /// Wikidata HTTP request timeout in seconds (default: 30)
    #[arg(long, global = true, default_value_t = 30u64, value_name = "SECONDS")]
    wikidata_timeout: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile one or more .tdsl files to IR JSON (multiple files are merged)
    Build {
        /// Input .tdsl file path(s); when multiple are given they are merged in order.
        /// Not required when --json-schema is specified.
        #[arg(value_name = "FILE", required = false, num_args = 0..)]
        inputs: Vec<PathBuf>,

        /// Output JSON file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Pretty-print JSON output
        #[arg(long, default_value_t = false)]
        pretty: bool,

        /// Skip Wikidata fetching (only process static items)
        #[arg(long, default_value_t = false)]
        offline: bool,

        /// Bypass the local Wikidata cache and force a fresh API request
        #[arg(long, default_value_t = false)]
        no_cache: bool,

        /// Cache time-to-live in seconds (0 disables caching, default: 86400 = 24h)
        #[arg(long, default_value_t = 86400u64)]
        cache_ttl: u64,

        /// Output the JSON Schema for TimelineIr to stdout (no input file required)
        #[arg(long, default_value_t = false)]
        json_schema: bool,
    },

    /// Merge multiple .tdsl files into a single IR JSON
    Merge {
        /// Input .tdsl file paths (merged in order; first file's meta takes precedence)
        #[arg(value_name = "FILE", required = true, num_args = 2..)]
        inputs: Vec<PathBuf>,

        /// Output JSON file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Pretty-print JSON output
        #[arg(long, default_value_t = false)]
        pretty: bool,

        /// Skip Wikidata fetching (only process static items)
        #[arg(long, default_value_t = false)]
        offline: bool,

        /// Bypass the local Wikidata cache and force a fresh API request
        #[arg(long, default_value_t = false)]
        no_cache: bool,

        /// Cache time-to-live in seconds (0 disables caching, default: 86400 = 24h)
        #[arg(long, default_value_t = 86400u64)]
        cache_ttl: u64,
    },

    /// Check a .tdsl file for syntax and semantic errors
    Check {
        /// Input .tdsl files or directories (directories are searched
        /// recursively for *.tdsl)
        #[arg(value_name = "FILE", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,

        /// Skip Wikidata resolution (currently the only mode; makes the
        /// offline-only behaviour explicit on the command line)
        #[arg(long)]
        offline: bool,

        /// Output format
        #[arg(long, value_enum, default_value_t = CheckOutputFormat::Text)]
        format: CheckOutputFormat,

        /// Exit non-zero when any warning is reported. CI-friendly.
        #[arg(long, default_value_t = false)]
        deny_warnings: bool,
    },

    /// Dump the parsed AST (for debugging)
    Ast {
        #[arg(value_name = "FILE")]
        input: PathBuf,
    },

    /// Fetch a Wikidata entity and display its data
    Fetch {
        /// Wikidata QID (e.g., Q7209)
        qid: String,

        /// Languages to fetch labels for
        #[arg(short, long, default_value = "ja,en")]
        lang: String,
    },

    /// Search Wikidata entities by keyword and list candidate QIDs
    Search {
        /// Free-text query (e.g., "漢王朝")
        query: String,

        /// Language used by Wikidata search
        #[arg(short, long, default_value = "ja")]
        lang: String,

        /// Max number of results (1..50)
        #[arg(short = 'n', long, default_value_t = 10)]
        limit: usize,

        /// Output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Inspect one Wikidata entity and suggest timeline mapping strategy
    Inspect {
        /// Wikidata QID (e.g., Q7209)
        qid: String,

        /// Label fallback languages (comma-separated)
        #[arg(short, long, default_value = "ja,en")]
        lang: String,

        /// Output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Resolve a Wikipedia article URL to a Wikidata QID
    Resolve {
        /// Wikipedia article URL
        url: String,

        /// Label fallback languages (comma-separated)
        #[arg(short, long, default_value = "ja,en")]
        lang: String,

        /// Output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Generate a .tdsl template from Wikidata entities
    Scaffold {
        #[command(subcommand)]
        target: ScaffoldTarget,
    },

    /// Render a .tdsl file to HTML, SVG, PNG, or PDF
    Render {
        /// Input .tdsl file path
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format
        #[arg(long, value_enum, default_value_t = RenderFormat::Html)]
        format: RenderFormat,

        /// Pixels per year on the horizontal axis
        #[arg(long, default_value_t = 2.0)]
        scale: f64,

        /// Height of each lane in pixels
        #[arg(long, default_value_t = 60.0)]
        lane_height: f64,

        /// Width of the left-hand gutter for lane labels
        #[arg(long, default_value_t = 120.0)]
        left_gutter: f64,

        /// Top margin reserved for the time axis
        #[arg(long, default_value_t = 40.0)]
        top_margin: f64,

        /// Color/style theme
        #[arg(long, default_value = "default", value_enum)]
        theme: ThemeArg,

        /// Path to a CSS file whose contents are injected after the theme CSS
        #[arg(long)]
        custom_css: Option<PathBuf>,

        /// Output DPI for PNG format (default 96). Scales pixel dimensions as dpi/96. Only applied with --format png.
        #[arg(long, conflicts_with = "png_scale")]
        dpi: Option<u32>,

        /// Fixed pixel scale multiplier for PNG format (e.g. 2.0 doubles dimensions). Overrides --dpi. Only applied with --format png.
        #[arg(long, conflicts_with = "dpi")]
        png_scale: Option<f64>,

        /// Enable interactive mode (zoom, pan, search, legend, detail panel)
        #[arg(long, default_value_t = false)]
        interactive: bool,

        /// Skip Wikidata fetching (only process static items)
        #[arg(long, default_value_t = false)]
        offline: bool,

        /// Bypass the local Wikidata cache and force a fresh API request
        #[arg(long, default_value_t = false)]
        no_cache: bool,

        /// Cache time-to-live in seconds (0 disables caching, default: 86400 = 24h)
        #[arg(long, default_value_t = 86400u64)]
        cache_ttl: u64,

        /// Tag-to-color mapping (e.g. "war=#cc0000,dynasty=#3366cc")
        #[arg(long)]
        color_map: Option<String>,

        /// Timeline orientation (horizontal or vertical)
        #[arg(long, value_enum, default_value_t = OrientationArg::Horizontal)]
        orientation: OrientationArg,

        /// Auxiliary grid lines: none (default), decade, year, or month
        #[arg(long, value_enum, default_value_t = GridStyleArg::None)]
        grid: GridStyleArg,

        /// High-level layout style: timeline (default), group-bands (era/group background
        /// blocks), gantt (emphasized month grid + always-on period labels), or zigzag
        /// (alternating up/down placement; only for <=2 lanes, otherwise returns an
        /// explicit error)
        #[arg(long, value_enum, default_value_t = LayoutStyleArg::Timeline)]
        layout_style: LayoutStyleArg,

        /// Watch input file for changes and re-render automatically (html/svg only)
        #[arg(long, default_value_t = false)]
        watch: bool,

        /// Append an item listing table after the SVG (all formats: html/svg/png/pdf)
        #[arg(long, default_value_t = false)]
        show_table: bool,

        /// Render a static legend panel showing lane and tag colors
        #[arg(long, default_value_t = false)]
        show_legend: bool,

        /// Always render labels next to event dots and event-range bars as SVG text
        #[arg(long, default_value_t = false)]
        show_event_labels: bool,

        /// PDF page size: a4 (default), a3, letter. Only applied with --format pdf.
        #[arg(long, value_enum, default_value_t = PdfPageSizeArg::A4)]
        pdf_size: PdfPageSizeArg,

        /// Use landscape (rotated) orientation for PDF output. Only applied with --format pdf.
        #[arg(long, default_value_t = false)]
        pdf_landscape: bool,

        /// PDF page margin in millimetres. Only applied with --format pdf.
        #[arg(long, default_value_t = 10.0)]
        pdf_margin: f64,

        /// Override the PDF document Title metadata (defaults to the timeline title). Only applied with --format pdf.
        #[arg(long)]
        pdf_title: Option<String>,

        /// Split the item table onto separate PDF pages (ADR-0004). Requires
        /// --show-table; only applied with --format pdf. Without
        /// --chart-pagination the timeline chart stays on a single page
        /// (ADR-0004 D1); with --chart-pagination the chart is instead split
        /// per --chart-pagination and the table pages always follow all
        /// chart pages (issue #661). Default: disabled (existing single-page
        /// table behavior is unchanged).
        #[arg(long, default_value_t = false)]
        pdf_pagination: bool,

        /// Split the chart into multiple pages by lane group, N lanes per
        /// page (issue #660/#661, ADR-0005 D2). Applies to --format svg
        /// (separate `<stem>.pageN.svg` files, zero-padded to the total page
        /// count's digit width) and --format pdf (separate PDF pages: all
        /// chart pages first, then any --show-table table page(s); when
        /// combined with --pdf-pagination, table page footers count only the
        /// table pages). Requires --output. Not compatible with --watch.
        /// Mutually exclusive with --chart-pagination-range. Default:
        /// disabled (single-page output unchanged).
        #[arg(long)]
        chart_pagination: Option<usize>,

        /// Split the chart into N pages along the time-range axis instead of
        /// by lane group (issue #733/#736/#734, ADR-0005 D3): `meta.range` is
        /// divided into N contiguous, non-empty integer-year segments and
        /// each is rendered as its own page. A `Span`/`EventRange` that
        /// straddles a segment boundary is clipped on each page it
        /// intersects, with a continuation-marker glyph drawn at the clipped
        /// edge (a warning is also printed listing every such item). Applies
        /// to --format svg (separate `<stem>.pageN.svg` files, zero-padded to
        /// the total page count's digit width) and --format pdf (separate
        /// PDF pages: all chart pages first, then any --show-table table
        /// page(s); when combined with --pdf-pagination, table page footers
        /// count only the table pages). Requires --output. Not compatible
        /// with --watch or --chart-pagination. Default: disabled (single-page
        /// output unchanged).
        #[arg(long)]
        chart_pagination_range: Option<usize>,
    },

    /// Generate a minimal .tdsl template for manual authoring
    Init {
        /// Output .tdsl file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Timeline display title
        #[arg(long, default_value = "新しい年表")]
        timeline: String,

        /// Range start year
        #[arg(long, default_value_t = 0)]
        range_start: i64,

        /// Range end year
        #[arg(long, default_value_t = 2000)]
        range_end: i64,

        /// Comma-separated lane labels (e.g. "王朝,事件,人物")
        #[arg(long, default_value = "")]
        lanes: String,
    },

    /// Import timeline items from CSV (`lane,type,start,end,time,label,tags,id`, plus optional
    /// `source` / `origin`).
    /// `start` / `end` / `time` columns accept `YYYY-MM-DD`, `YYYY-MM`, or `YYYY` (negative years are year-precision only).
    /// `source` must be `<ident>:<QID>` (e.g. `wd:Q7209`) and `origin` must be a valid DSL ident;
    /// `origin=wikidata` requires a `wd:Q<id>` source (#608).
    ImportCsv {
        /// Input CSV file path (UTF-8 with header row)
        #[arg(value_name = "CSV")]
        input: PathBuf,

        /// Output .tdsl snippet path (default: stdout)
        #[arg(short, long, conflicts_with = "append")]
        output: Option<PathBuf>,

        /// Append generated items to an existing .tdsl file
        #[arg(long, conflicts_with = "output")]
        append: Option<PathBuf>,
    },

    /// Export timeline items from IR to CSV (`lane,type,start,end,time,label,tags,id,source,origin`).
    /// Symmetric with `import-csv`: re-importing all 10 columns (including `source` / `origin`,
    /// #608) yields a semantically equal IR. Output is generated from the IR (single source of
    /// truth), not the parser.
    ExportCsv {
        /// Input `.tdsl` source or `.json` IR file path
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Output CSV file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Skip Wikidata fetching (only export static items). Ignored for `.json` IR input.
        #[arg(long, default_value_t = false)]
        offline: bool,

        /// Bypass the local Wikidata cache and force a fresh API request
        #[arg(long, default_value_t = false)]
        no_cache: bool,

        /// Cache time-to-live in seconds (0 disables caching, default: 86400 = 24h)
        #[arg(long, default_value_t = 86400u64)]
        cache_ttl: u64,
    },

    /// Format a .tdsl file with canonical style (2-space indent, blank line between blocks).
    ///
    /// By default the formatted source is written to stdout.
    /// Use --write to update the file in-place, or --check (for CI) to exit non-zero when
    /// the file is not already formatted.
    ///
    /// NOTE: top-level comments (`//` and `/* */`) are preserved in place. Comments inside
    /// blocks are kept (content is not lost) but relocated to the nearest block boundary.
    /// `tdsl decompile` cannot restore comments because it starts from the IR.
    Fmt {
        /// Input .tdsl files or directories (directories are searched
        /// recursively for *.tdsl)
        #[arg(value_name = "FILE", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,

        /// Exit non-zero when the file is not formatted (do not modify the file). CI-friendly.
        #[arg(long, default_value_t = false, conflicts_with = "write")]
        check: bool,

        /// Overwrite the file with the formatted source in-place.
        #[arg(long, default_value_t = false, conflicts_with = "check")]
        write: bool,
    },

    /// Lint a .tdsl file and optionally apply safe fixes
    Lint {
        /// Input .tdsl files or directories (directories are searched
        /// recursively for *.tdsl)
        #[arg(value_name = "FILE", required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,

        /// Apply safe fixes in-place
        #[arg(long, default_value_t = false)]
        fix: bool,

        /// Output format
        #[arg(long, value_enum, default_value_t = LintOutputFormat::Text)]
        format: LintOutputFormat,
    },

    /// Manage the local Wikidata cache (~/.cache/tdsl/)
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Decompile a JSON IR file back to a .tdsl source file
    Decompile {
        /// Input JSON file path (default: stdin)
        input: Option<PathBuf>,

        /// Output .tdsl file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate shell completion scripts (bash / fish / zsh / powershell / elvish)
    Completions {
        /// Target shell
        shell: clap_complete::Shell,
    },

    /// Start a Language Server Protocol server over stdio
    ///
    /// Communicates via stdin/stdout using the LSP JSON-RPC protocol.
    /// Supported features: textDocument/publishDiagnostics (parse errors + validation
    /// warnings), completion (DSL keyword completion), hover (lane ID -> lane info,
    /// QID -> cached entity info), definition, references, rename, documentSymbol,
    /// codeAction (lint --fix quick fixes), and formatting (canonical source
    /// formatting; comments are not preserved).
    Lsp,
}

#[derive(Subcommand)]
enum CacheAction {
    /// Show cache statistics (file count, total size, oldest/newest entry)
    Status,

    /// Delete cache entries
    Clear {
        /// Only delete entries older than this many days
        #[arg(long)]
        older_than: Option<u64>,
    },
}

#[derive(Subcommand)]
enum ScaffoldTarget {
    /// Scaffold a timeline from a list of Wikidata QIDs
    Wikidata {
        /// Comma-separated QIDs (e.g., Q7183,Q7209)
        #[arg(long)]
        qids: String,

        /// Timeline display title
        #[arg(long)]
        timeline: String,

        /// Output .tdsl file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Label fallback languages (comma-separated)
        #[arg(short, long, default_value = "ja,en")]
        lang: String,

        /// Mapping target strategy
        #[arg(long, value_enum, default_value_t = ScaffoldTargetType::Auto)]
        target: ScaffoldTargetType,

        /// Lane assignment strategy
        #[arg(long, value_enum, default_value_t = ScaffoldLaneMode::PerEntity)]
        lane_mode: ScaffoldLaneMode,

        /// Label of the shared lane used when lane-mode=single
        #[arg(long, default_value = "項目")]
        single_lane_label: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum ScaffoldTargetType {
    #[default]
    Auto,
    Span,
    Event,
    EventRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum ScaffoldLaneMode {
    Single,
    #[default]
    PerEntity,
    ByKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum LintOutputFormat {
    Text,
    Json,
}

/// `check --format` の出力形式。`lint` と同じ選択肢に揃える（#748）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum CheckOutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum ThemeArg {
    #[default]
    Default,
    Dark,
    Print,
    Pastel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum OrientationArg {
    #[default]
    Horizontal,
    Vertical,
}

impl OrientationArg {
    fn into_orientation(self) -> tdsl_render::layout::Orientation {
        match self {
            OrientationArg::Horizontal => tdsl_render::layout::Orientation::Horizontal,
            OrientationArg::Vertical => tdsl_render::layout::Orientation::Vertical,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum GridStyleArg {
    #[default]
    None,
    Decade,
    Year,
    Month,
}

impl GridStyleArg {
    fn into_grid_style(self) -> tdsl_render::layout::GridStyle {
        match self {
            GridStyleArg::None => tdsl_render::layout::GridStyle::None,
            GridStyleArg::Decade => tdsl_render::layout::GridStyle::Decade,
            GridStyleArg::Year => tdsl_render::layout::GridStyle::Year,
            GridStyleArg::Month => tdsl_render::layout::GridStyle::Month,
        }
    }
}

/// High-level visual layout style (#543), orthogonal to `--orientation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum LayoutStyleArg {
    #[default]
    Timeline,
    /// Draw background blocks spanning contiguous lane groups/eras.
    GroupBands,
    /// Project-management-style layout (#564): emphasized month grid +
    /// always-on Span/EventRange period labels.
    Gantt,
    /// Alternating up/down (zigzag) placement of items within a single lane
    /// (#565), sorted by start time. Supported only when the timeline has at
    /// most 2 lanes; otherwise rendering returns an explicit error.
    Zigzag,
}

impl LayoutStyleArg {
    fn into_layout_style(self) -> tdsl_render::layout::LayoutStyle {
        match self {
            LayoutStyleArg::Timeline => tdsl_render::layout::LayoutStyle::Timeline,
            LayoutStyleArg::GroupBands => tdsl_render::layout::LayoutStyle::GroupBands,
            LayoutStyleArg::Gantt => tdsl_render::layout::LayoutStyle::Gantt,
            LayoutStyleArg::Zigzag => tdsl_render::layout::LayoutStyle::Zigzag,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Default, Debug)]
enum RenderFormat {
    #[default]
    Html,
    Svg,
    Png,
    Pdf,
}

#[derive(ValueEnum, Clone, Copy, Default, Debug)]
enum PdfPageSizeArg {
    #[default]
    A4,
    A3,
    Letter,
}

impl PdfPageSizeArg {
    fn into_page_size(self) -> tdsl_render::PdfPageSize {
        match self {
            PdfPageSizeArg::A4 => tdsl_render::PdfPageSize::A4,
            PdfPageSizeArg::A3 => tdsl_render::PdfPageSize::A3,
            PdfPageSizeArg::Letter => tdsl_render::PdfPageSize::Letter,
        }
    }
}

impl ThemeArg {
    fn into_theme(self) -> tdsl_render::layout::Theme {
        match self {
            ThemeArg::Default => tdsl_render::layout::Theme::Default,
            ThemeArg::Dark => tdsl_render::layout::Theme::Dark,
            ThemeArg::Print => tdsl_render::layout::Theme::Print,
            ThemeArg::Pastel => tdsl_render::layout::Theme::Pastel,
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let wikidata_timeout = std::time::Duration::from_secs(cli.wikidata_timeout);

    let result = match cli.command {
        Commands::Build {
            inputs,
            output,
            pretty,
            offline,
            no_cache,
            cache_ttl,
            json_schema,
        } => {
            if json_schema {
                commands::build::cmd_json_schema(output.as_deref(), pretty)
            } else {
                commands::build::cmd_build(
                    &inputs,
                    output.as_deref(),
                    pretty,
                    offline,
                    tdsl_wikidata::CacheOptions {
                        no_cache,
                        ttl: std::time::Duration::from_secs(cache_ttl),
                    },
                    wikidata_timeout,
                )
            }
        }
        Commands::Merge {
            inputs,
            output,
            pretty,
            offline,
            no_cache,
            cache_ttl,
        } => commands::build::cmd_build(
            &inputs,
            output.as_deref(),
            pretty,
            offline,
            tdsl_wikidata::CacheOptions {
                no_cache,
                ttl: std::time::Duration::from_secs(cache_ttl),
            },
            wikidata_timeout,
        ),
        Commands::Check {
            inputs,
            offline,
            format,
            deny_warnings,
        } => commands::resolve_tdsl_inputs(&inputs).and_then(|files| {
            // JSON は全件を 1 つの配列にまとめる（単一入力はオブジェクト）。
            // 理由は lint と同じ（#750）。
            let mut reports = Vec::new();
            let json = matches!(format, CheckOutputFormat::Json);
            let result = commands::run_over_inputs(&files, |path| {
                let sink = if json { Some(&mut reports) } else { None };
                commands::check::cmd_check(path, offline, format, deny_warnings, sink)
            });
            if json {
                let rendered = match (files.len(), reports.as_slice()) {
                    (1, [only]) => Some(serde_json::to_string_pretty(only)),
                    (1, []) => None,
                    _ => Some(serde_json::to_string_pretty(&reports)),
                };
                if let Some(rendered) = rendered {
                    println!("{}", rendered.map_err(|e| e.to_string())?);
                }
            }
            result
        }),
        Commands::Ast { input } => commands::check::cmd_ast(&input),
        Commands::Fetch { qid, lang } => commands::fetch::cmd_fetch(&qid, &lang, wikidata_timeout),
        Commands::Search {
            query,
            lang,
            limit,
            json,
        } => commands::fetch::cmd_search(&query, &lang, limit, json, wikidata_timeout),
        Commands::Inspect { qid, lang, json } => {
            commands::fetch::cmd_inspect(&qid, &lang, json, wikidata_timeout)
        }
        Commands::Resolve { url, lang, json } => {
            commands::fetch::cmd_resolve(&url, &lang, json, wikidata_timeout)
        }
        Commands::Scaffold { target } => match target {
            ScaffoldTarget::Wikidata {
                qids,
                timeline,
                output,
                lang,
                target,
                lane_mode,
                single_lane_label,
            } => {
                commands::scaffold::cmd_scaffold_wikidata(commands::scaffold::ScaffoldCliOptions {
                    qids: &qids,
                    timeline: &timeline,
                    output: output.as_deref(),
                    lang: &lang,
                    target,
                    lane_mode,
                    single_lane_label: &single_lane_label,
                    wikidata_timeout,
                })
            }
        },
        Commands::Render {
            input,
            output,
            format,
            scale,
            lane_height,
            left_gutter,
            top_margin,
            theme,
            custom_css,
            dpi,
            png_scale,
            interactive,
            offline,
            no_cache,
            cache_ttl,
            color_map,
            orientation,
            grid,
            layout_style,
            watch,
            show_table,
            show_legend,
            show_event_labels,
            pdf_size,
            pdf_landscape,
            pdf_margin,
            pdf_title,
            pdf_pagination,
            chart_pagination,
            chart_pagination_range,
        } => commands::render::cmd_render(
            &input,
            output.as_deref(),
            watch,
            commands::render::RenderCliOptions {
                format,
                scale,
                lane_height,
                left_gutter,
                top_margin,
                theme,
                custom_css_path: custom_css.as_deref(),
                dpi,
                png_scale,
                interactive,
                offline,
                cache_opts: tdsl_wikidata::CacheOptions {
                    no_cache,
                    ttl: std::time::Duration::from_secs(cache_ttl),
                },
                color_map_raw: color_map.as_deref(),
                orientation,
                grid,
                layout_style,
                wikidata_timeout,
                show_table,
                show_legend,
                show_event_labels,
                pdf_cli: commands::render::PdfCliOptions {
                    size: pdf_size.into_page_size(),
                    landscape: pdf_landscape,
                    margin_mm: pdf_margin,
                    title: pdf_title,
                    pagination: pdf_pagination,
                },
                chart_pagination,
                chart_pagination_range,
            },
        ),
        Commands::Init {
            output,
            timeline,
            range_start,
            range_end,
            lanes,
        } => commands::init::cmd_init(output.as_deref(), &timeline, range_start, range_end, &lanes),
        Commands::ImportCsv {
            input,
            output,
            append,
        } => commands::init::cmd_import_csv(&input, output.as_deref(), append.as_deref()),
        Commands::ExportCsv {
            input,
            output,
            offline,
            no_cache,
            cache_ttl,
        } => commands::export_csv::cmd_export_csv(
            &input,
            output.as_deref(),
            offline,
            tdsl_wikidata::CacheOptions {
                no_cache,
                ttl: std::time::Duration::from_secs(cache_ttl),
            },
            wikidata_timeout,
        ),
        Commands::Fmt {
            inputs,
            check,
            write,
        } => commands::resolve_tdsl_inputs(&inputs).and_then(|files| {
            commands::run_over_inputs(&files, |path| commands::fmt::cmd_fmt(path, check, write))
        }),
        Commands::Lint {
            inputs,
            fix,
            format,
        } => commands::resolve_tdsl_inputs(&inputs).and_then(|files| {
            // JSON 出力は全件を 1 つの配列にまとめて出す。ファイルごとに print
            // するとオブジェクトが連結され、単一の JSON 文書として不正になる
            // （#750）。**失敗したファイルのレポートも配列に残す。**
            let mut reports = Vec::new();
            let json = matches!(format, LintOutputFormat::Json);
            let result = commands::run_over_inputs(&files, |path| {
                let sink = if json { Some(&mut reports) } else { None };
                commands::lint::cmd_lint(path, fix, format, sink)
            });
            if json {
                // **単一ファイル指定時は従来どおりオブジェクトを出す。**
                // 常に配列にすると、既存の消費側（`jq '.issue_count'` など）が
                // 黙って壊れる。複数入力は新しい使い方なので配列でよい。
                // **`reports` は空になりうる。** パースエラーで落ちたファイルは
                // レポートを積まないため、`reports[0]` を直接引くと panic する
                // （実際に踏んだ）。単一ファイルでも要素が無ければ何も出さない。
                let rendered = match (files.len(), reports.as_slice()) {
                    (1, [only]) => Some(serde_json::to_string_pretty(only)),
                    (1, []) => None,
                    _ => Some(serde_json::to_string_pretty(&reports)),
                };
                if let Some(rendered) = rendered {
                    println!("{}", rendered.map_err(|e| e.to_string())?);
                }
            }
            result
        }),
        Commands::Cache { action } => commands::cache::cmd_cache(action),
        Commands::Decompile { input, output } => {
            commands::decompile::cmd_decompile(input.as_deref(), output.as_deref())
        }
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "tdsl", &mut std::io::stdout());
            Ok(())
        }
        Commands::Lsp => commands::lsp::cmd_lsp(),
    };

    if let Err(e) = result {
        // ParseError は commands::check::print_parse_error で miette 出力済みのため
        // 空文字列の場合は重複出力を避けて終了コードだけ設定する。
        if !e.is_empty() {
            eprintln!("Error: {e}");
        }
        process::exit(1);
    }
}
