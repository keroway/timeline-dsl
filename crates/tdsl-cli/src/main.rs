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
        /// Input .tdsl file path(s); when multiple are given they are merged in order
        #[arg(value_name = "FILE", required = true, num_args = 1..)]
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
        #[arg(value_name = "FILE")]
        input: PathBuf,
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

    /// Render a .tdsl file to a standalone HTML timeline
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

    /// Import timeline items from CSV (`lane,type,start,end,time,label,tags,id`).
    /// `start` / `end` / `time` columns accept `YYYY-MM-DD`, `YYYY-MM`, or `YYYY` (negative years are year-precision only)
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

    /// Lint a .tdsl file and optionally apply safe fixes
    Lint {
        /// Input .tdsl file path
        #[arg(value_name = "FILE")]
        input: PathBuf,

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

    /// Start a Language Server Protocol server over stdio (Diagnostics + Completion + Hover)
    ///
    /// Communicates via stdin/stdout using the LSP JSON-RPC protocol.
    /// Supported features: textDocument/publishDiagnostics (parse errors + validation warnings),
    /// textDocument/completion (DSL keyword completion), and textDocument/hover
    /// (lane ID -> lane info, QID -> cached entity info).
    /// Goto Definition and Code Actions will be added in future issues.
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

#[derive(ValueEnum, Clone, Default, Debug)]
enum RenderFormat {
    #[default]
    Html,
    Svg,
    Png,
    Pdf,
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
        Commands::Check { input } => commands::check::cmd_check(&input),
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
            } => commands::scaffold::cmd_scaffold_wikidata(
                &qids,
                &timeline,
                output.as_deref(),
                &lang,
                target,
                lane_mode,
                &single_lane_label,
                wikidata_timeout,
            ),
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
        } => commands::render::cmd_render(
            &input,
            output.as_deref(),
            format,
            scale,
            lane_height,
            left_gutter,
            top_margin,
            theme,
            custom_css.as_deref(),
            dpi,
            png_scale,
            interactive,
            offline,
            tdsl_wikidata::CacheOptions {
                no_cache,
                ttl: std::time::Duration::from_secs(cache_ttl),
            },
            color_map.as_deref(),
            orientation,
            wikidata_timeout,
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
        Commands::Lint { input, fix, format } => commands::lint::cmd_lint(&input, fix, format),
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
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
