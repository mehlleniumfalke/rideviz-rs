use std::sync::OnceLock;

use crate::error::RasterError;
use crate::types::viz::OutputConfig;

static FONT_DB: OnceLock<usvg::fontdb::Database> = OnceLock::new();

pub fn rasterize(svg: &str, config: &OutputConfig) -> Result<Vec<u8>, RasterError> {
    let fontdb = FONT_DB.get_or_init(load_font_db);
    rasterize_with_fontdb(svg, config, fontdb)
}

fn load_font_db() -> usvg::fontdb::Database {
    let mut fontdb = usvg::fontdb::Database::new();
    for path in [
        "/app/assets/fonts/Geist-Regular.otf",
        "./assets/fonts/Geist-Regular.otf",
        "/app/assets/fonts/GeistPixel-Square.ttf",
        "./assets/fonts/GeistPixel-Square.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ] {
        let _ = fontdb.load_font_file(path);
    }
    fontdb
}

fn rasterize_with_fontdb(
    svg: &str,
    config: &OutputConfig,
    fontdb: &usvg::fontdb::Database,
) -> Result<Vec<u8>, RasterError> {
    let svg = if config.watermark {
        inject_watermark(svg, config.width, config.height)
    } else {
        svg.to_string()
    };

    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg, &options, fontdb)
        .map_err(|e| RasterError::RenderFailed(format!("Failed to parse SVG: {}", e)))?;

    let mut pixmap = tiny_skia::Pixmap::new(config.width, config.height)
        .ok_or_else(|| RasterError::RenderFailed("Failed to create pixmap".to_string()))?;

    if let Some((r, g, b, a)) = config.background {
        pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, a));
    }

    let transform = tiny_skia::Transform::from_scale(
        config.width as f32 / tree.size().width(),
        config.height as f32 / tree.size().height(),
    );

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|e| RasterError::RenderFailed(format!("Failed to encode PNG: {}", e)))
}

fn inject_watermark(
    svg: &str,
    width: u32,
    height: u32,
) -> String {
    const WATERMARK_TEXT: &str = "rideviz.online";
    const WATERMARK_ID: &str = "rideviz-watermark";

    if svg.contains(WATERMARK_ID) {
        return svg.to_string();
    }

    let font_size = ((height as f32 * 0.020) as u32).max(13);
    let padding_x = ((font_size as f32 * 0.7) as u32).max(8);
    let padding_y = ((font_size as f32 * 0.5) as u32).max(6);
    let margin_bottom = ((font_size as f32 * 1.15) as u32).max(16);

    let text_x = width / 2;
    let text_y = height.saturating_sub(margin_bottom);

    let approx_text_width = (WATERMARK_TEXT.chars().count() as f32) * (font_size as f32) * 0.62;
    let box_width = ((approx_text_width + (padding_x * 2) as f32).ceil() as u32).min(width.saturating_sub(12));
    let box_height = (font_size + padding_y * 2).min(height.saturating_sub(12));
    let box_x = text_x.saturating_sub(box_width / 2);
    let box_y = text_y.saturating_sub(font_size + padding_y);
    let radius = ((font_size as f32 * 0.3).round() as u32).clamp(3, 6);

    let border_width = ((font_size as f32 * 0.08).round() as u32).max(1);

    let nodes = format!(
        "<g id=\"{WATERMARK_ID}\">\
<rect x=\"{box_x}\" y=\"{box_y}\" width=\"{box_width}\" height=\"{box_height}\" rx=\"{radius}\" fill=\"rgb(255,255,255)\" fill-opacity=\"0.90\" stroke=\"rgb(0,0,0)\" stroke-opacity=\"0.92\" stroke-width=\"{border_width}\" />\
<text x=\"{text_x}\" y=\"{text_y}\" font-family=\"Geist Pixel, DejaVu Sans Mono, DejaVu Sans, sans-serif\" font-size=\"{font_size}\" text-anchor=\"middle\" fill=\"rgb(0,0,0)\" fill-opacity=\"0.92\">{WATERMARK_TEXT}</text>\
</g>"
    );

    if svg.contains("</svg>") {
        svg.replacen("</svg>", &format!("{nodes}</svg>"), 1)
    } else {
        format!("{svg}{nodes}")
    }
}

#[cfg(test)]
mod tests {
    use super::inject_watermark;

    #[test]
    fn injects_a_single_watermark_group() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\"></svg>";
        let injected = inject_watermark(svg, 100, 100);
        assert!(injected.contains("id=\"rideviz-watermark\""));
        assert!(injected.contains("rideviz.online"));

        let injected_twice = inject_watermark(&injected, 100, 100);
        assert_eq!(
            injected_twice.matches("id=\"rideviz-watermark\"").count(),
            1
        );
    }
}
