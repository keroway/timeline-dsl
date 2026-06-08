use crate::{GridStyleArg, OrientationArg, RenderFormat, ThemeArg};

/// PDF-specific CLI options bundled together to reduce argument count.
pub(crate) struct PdfCliOptions {
    pub size: tdsl_render::PdfPageSize,
    pub landscape: bool,
    pub margin_mm: f64,
    pub title: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_render(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    format: RenderFormat,
    scale: f64,
    lane_height: f64,
    left_gutter: f64,
    top_margin: f64,
    theme: ThemeArg,
    custom_css_path: Option<&std::path::Path>,
    dpi: Option<u32>,
    png_scale: Option<f64>,
    interactive: bool,
    offline: bool,
    cache_opts: tdsl_wikidata::CacheOptions,
    color_map_raw: Option<&str>,
    orientation: OrientationArg,
    grid: GridStyleArg,
    wikidata_timeout: std::time::Duration,
    watch: bool,
    show_table: bool,
    show_event_labels: bool,
    pdf_cli: PdfCliOptions,
) -> Result<(), String> {
    if watch {
        let out_path = output.ok_or(
            "--watch requires --output <file>; stdout is not supported in watch mode".to_string(),
        )?;
        match format {
            RenderFormat::Png | RenderFormat::Pdf => {
                return Err("--watch supports --format html or svg only (not png/pdf)".to_string());
            }
            _ => {}
        }
        if !offline {
            eprintln!(
                "Note: watch mode re-fetches Wikidata on every change. Consider --offline for faster iteration."
            );
        }
        // pdf_cli is not passed to watch mode as pdf is not supported there.
        return cmd_render_watch(
            input,
            out_path,
            format,
            scale,
            lane_height,
            left_gutter,
            top_margin,
            theme,
            custom_css_path,
            interactive,
            offline,
            cache_opts,
            color_map_raw,
            orientation,
            grid,
            wikidata_timeout,
            show_table,
            show_event_labels,
        );
    }

    do_render(
        input,
        output,
        format,
        scale,
        lane_height,
        left_gutter,
        top_margin,
        theme,
        custom_css_path,
        dpi,
        png_scale,
        interactive,
        offline,
        cache_opts,
        color_map_raw,
        orientation,
        grid,
        wikidata_timeout,
        show_table,
        show_event_labels,
        pdf_cli,
    )
}

#[allow(clippy::too_many_arguments)]
fn do_render(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    format: RenderFormat,
    scale: f64,
    lane_height: f64,
    left_gutter: f64,
    top_margin: f64,
    theme: ThemeArg,
    custom_css_path: Option<&std::path::Path>,
    dpi: Option<u32>,
    png_scale: Option<f64>,
    interactive: bool,
    offline: bool,
    cache_opts: tdsl_wikidata::CacheOptions,
    color_map_raw: Option<&str>,
    orientation: OrientationArg,
    grid: GridStyleArg,
    wikidata_timeout: std::time::Duration,
    show_table: bool,
    show_event_labels: bool,
    pdf_cli: PdfCliOptions,
) -> Result<(), String> {
    let ir = super::build::load_ir(input, offline, cache_opts, wikidata_timeout)?;

    let custom_css = match custom_css_path {
        Some(path) => {
            let css = std::fs::read_to_string(path)
                .map_err(|e| format!("Failed to read CSS file {}: {e}", path.display()))?;
            Some(css)
        }
        None => None,
    };

    let mut color_map = ir.meta.color_map.clone();
    if let Some(raw) = color_map_raw {
        for (tag, color) in parse_color_map(raw)? {
            color_map.insert(tag, color);
        }
    }

    // --show-table only applies to HTML output; emit a notice for other formats.
    let effective_show_table = match format {
        RenderFormat::Html => show_table,
        _ => {
            if show_table {
                eprintln!("Note: --show-table is only supported with --format html; ignoring.");
            }
            false
        }
    };

    let opts = tdsl_render::RenderOptions {
        scale,
        lane_height,
        left_gutter,
        top_margin,
        theme: theme.into_theme(),
        custom_css,
        color_map,
        interactive,
        orientation: orientation.into_orientation(),
        grid: grid.into_grid_style(),
        show_table: effective_show_table,
        show_event_labels,
        ..Default::default()
    };

    match format {
        RenderFormat::Html => {
            let html = tdsl_render::render_html(&ir, opts)
                .map_err(|e| format!("HTML rendering failed: {e}"))?;
            write_render_text(&html, output)
        }
        RenderFormat::Svg => {
            let svg = tdsl_render::render_svg_only(&ir, opts)
                .map_err(|e| format!("SVG rendering failed: {e}"))?;
            write_render_text(&svg, output)
        }
        RenderFormat::Png => {
            let png_opts = tdsl_render::PngOptions {
                dpi: dpi.unwrap_or(96),
                scale_factor: png_scale,
            };
            let bytes = tdsl_render::render_png(&ir, opts, png_opts)
                .map_err(|e| format!("PNG rendering failed: {e}"))?;
            write_render_binary(&bytes, output)
        }
        RenderFormat::Pdf => {
            let pdf_opts = tdsl_render::PdfOptions {
                page_size: pdf_cli.size,
                landscape: pdf_cli.landscape,
                margin_mm: pdf_cli.margin_mm,
                // When --pdf-title is not given, render_pdf fills in ir.meta.title.
                title: pdf_cli.title,
                creation_date: today_pdf_date(),
            };
            let bytes = tdsl_render::render_pdf(&ir, opts, pdf_opts)
                .map_err(|e| format!("PDF rendering failed: {e}"))?;
            write_render_binary(&bytes, output)
        }
    }
}

/// Derive today's date (UTC) as a [`tdsl_render::PdfDate`] for use in PDF
/// CreationDate metadata. Falls back to 1970-01-01 if the system clock is
/// unavailable rather than panicking.
fn today_pdf_date() -> Option<tdsl_render::PdfDate> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // Civil date from Unix timestamp (Howard Hinnant algorithm).
    // Reference: https://howardhinnant.github.io/date_algorithms.html#civil_from_days
    let days = secs / 86400;
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };

    Some(tdsl_render::PdfDate {
        year: (y as u16).min(9999),
        month: mo as u8,
        day: d as u8,
    })
}

#[allow(clippy::too_many_arguments)]
fn cmd_render_watch(
    input: &std::path::Path,
    output: &std::path::Path,
    format: RenderFormat,
    scale: f64,
    lane_height: f64,
    left_gutter: f64,
    top_margin: f64,
    theme: ThemeArg,
    custom_css_path: Option<&std::path::Path>,
    interactive: bool,
    offline: bool,
    cache_opts: tdsl_wikidata::CacheOptions,
    color_map_raw: Option<&str>,
    orientation: OrientationArg,
    grid: GridStyleArg,
    wikidata_timeout: std::time::Duration,
    show_table: bool,
    show_event_labels: bool,
) -> Result<(), String> {
    let render_once = |cache_opts: tdsl_wikidata::CacheOptions| {
        // Watch mode does not support PDF format (guarded before this call).
        // Pass a default PdfCliOptions; it is never used.
        do_render(
            input,
            Some(output),
            format,
            scale,
            lane_height,
            left_gutter,
            top_margin,
            theme,
            custom_css_path,
            None,
            None,
            interactive,
            offline,
            cache_opts,
            color_map_raw,
            orientation,
            grid,
            wikidata_timeout,
            show_table,
            show_event_labels,
            PdfCliOptions {
                size: tdsl_render::PdfPageSize::A4,
                landscape: false,
                margin_mm: 10.0,
                title: None,
            },
        )
    };

    render_once(cache_opts.clone())?;
    eprintln!(
        "Watching {} for changes. Press Ctrl+C to stop.",
        input.display()
    );

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        // channel error means receiver is gone; ignore
        let _ = tx.send(res);
    })
    .map_err(|e| format!("Failed to create file watcher: {e}"))?;

    use notify::Watcher;
    watcher
        .watch(input, notify::RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch {}: {e}", input.display()))?;

    for event_result in rx {
        match event_result {
            Ok(event) => {
                use notify::EventKind;
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        eprintln!("Change detected, re-rendering...");
                        match render_once(cache_opts.clone()) {
                            Ok(()) => eprintln!("Updated {}", output.display()),
                            Err(e) => eprintln!("Render error: {e}"),
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => eprintln!("Watch error: {e}"),
        }
    }

    Ok(())
}

fn parse_color_map(raw: &str) -> Result<std::collections::HashMap<String, String>, String> {
    let mut map = std::collections::HashMap::new();
    for pair in raw.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (tag, color) = pair
            .split_once('=')
            .ok_or_else(|| format!("Invalid color-map entry (expected tag=color): {pair}"))?;
        let tag = tag.trim().to_string();
        let color = color.trim().to_string();
        if !color.starts_with('#') || (color.len() != 4 && color.len() != 7) {
            eprintln!(
                "Warning: color '{color}' for tag '{tag}' is not a valid hex color (#RGB or #RRGGBB); skipping"
            );
            continue;
        }
        map.insert(tag, color);
    }
    Ok(map)
}

pub(crate) fn write_render_text(
    body: &str,
    output: Option<&std::path::Path>,
) -> Result<(), String> {
    if let Some(out_path) = output {
        std::fs::write(out_path, body)
            .map_err(|e| format!("Failed to write {}: {e}", out_path.display()))?;
        eprintln!("Written to {}", out_path.display());
    } else {
        println!("{body}");
    }
    Ok(())
}

pub(crate) fn write_render_binary(
    bytes: &[u8],
    output: Option<&std::path::Path>,
) -> Result<(), String> {
    use std::io::Write;
    if let Some(out_path) = output {
        std::fs::write(out_path, bytes)
            .map_err(|e| format!("Failed to write {}: {e}", out_path.display()))?;
        eprintln!("Written to {}", out_path.display());
    } else {
        std::io::stdout()
            .write_all(bytes)
            .map_err(|e| format!("Failed to write to stdout: {e}"))?;
    }
    Ok(())
}
