use crate::{GridStyleArg, OrientationArg, RenderFormat, ThemeArg};

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
    watch: bool,
    wikidata_timeout: std::time::Duration,
) -> Result<(), String> {
    if watch {
        // watch モードでは stdout 出力は無意味なのでエラーにする
        let out_path = output.ok_or_else(|| {
            "--watch を使用するには --output でファイルパスを指定してください".to_string()
        })?;

        // watch モードは HTML / SVG のみサポート
        match format {
            RenderFormat::Png | RenderFormat::Pdf => {
                return Err("--watch は --format html または svg のみサポートします".to_string());
            }
            RenderFormat::Html | RenderFormat::Svg => {}
        }

        // Wikidata フェッチが有効な場合に警告を出す（変更のたびに fetch が走るため）
        if !offline {
            eprintln!(
                "Warning: watch モードで Wikidata フェッチが有効です。変更のたびに API \
                 呼び出しが発生します。開発中は --offline の使用を推奨します。"
            );
        }

        cmd_render_watch(
            input,
            out_path,
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
        )
    } else {
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
        )
    }
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
    dpi: Option<u32>,
    png_scale: Option<f64>,
    interactive: bool,
    offline: bool,
    cache_opts: tdsl_wikidata::CacheOptions,
    color_map_raw: Option<&str>,
    orientation: OrientationArg,
    grid: GridStyleArg,
    wikidata_timeout: std::time::Duration,
) -> Result<(), String> {
    // 初回レンダリング
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
        dpi,
        png_scale,
        interactive,
        offline,
        cache_opts.clone(),
        color_map_raw,
        orientation,
        grid,
        wikidata_timeout,
    )?;

    eprintln!(
        "Watching {} for changes. Press Ctrl+C to stop.",
        input.display()
    );

    let (tx, rx) = std::sync::mpsc::channel();

    // notify::recommended_watcher は FnMut を受け取る
    let mut watcher = notify::recommended_watcher(move |res| {
        // チャネルが閉じられていた場合は無視する
        let _ = tx.send(res);
    })
    .map_err(|e| format!("Failed to create file watcher: {e}"))?;

    use notify::Watcher;
    watcher
        .watch(input, notify::RecursiveMode::NonRecursive)
        .map_err(|e| format!("Failed to watch {}: {e}", input.display()))?;

    for res in &rx {
        match res {
            Ok(event) => {
                use notify::EventKind;
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        eprintln!("Change detected, re-rendering...");
                        match do_render(
                            input,
                            Some(output),
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
                            cache_opts.clone(),
                            color_map_raw,
                            orientation,
                            grid,
                            wikidata_timeout,
                        ) {
                            Ok(()) => eprintln!("Updated {}", output.display()),
                            // レンダリングエラーは watch を継続する（ファイルが保存途中の場合がある）
                            Err(e) => eprintln!("Render error: {e}"),
                        }
                    }
                    // Remove / Access / Other 等はスキップ
                    _ => {}
                }
            }
            Err(e) => eprintln!("Watch error: {e}"),
        }
    }

    Ok(())
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
            let bytes = tdsl_render::render_pdf(&ir, opts, tdsl_render::PdfOptions::default())
                .map_err(|e| format!("PDF rendering failed: {e}"))?;
            write_render_binary(&bytes, output)
        }
    }
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
