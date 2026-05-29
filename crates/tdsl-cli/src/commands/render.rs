use crate::{OrientationArg, RenderFormat, ThemeArg};

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
        ..Default::default()
    };

    match format {
        RenderFormat::Html => write_render_text(&tdsl_render::render_html(&ir, opts), output),
        RenderFormat::Svg => write_render_text(&tdsl_render::render_svg_only(&ir, opts), output),
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
