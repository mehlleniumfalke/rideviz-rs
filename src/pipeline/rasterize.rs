use std::cell::RefCell;

use crate::error::RasterError;
use crate::types::viz::OutputConfig;

thread_local! {
    static FONT_DB: RefCell<usvg::fontdb::Database> = RefCell::new(load_font_db());
}

pub fn rasterize(svg: &str, config: &OutputConfig) -> Result<Vec<u8>, RasterError> {
    FONT_DB.with(|fontdb| {
        let fontdb = fontdb.borrow();
        rasterize_with_fontdb(svg, config, &fontdb)
    })
}

fn load_font_db() -> usvg::fontdb::Database {
    let mut fontdb = usvg::fontdb::Database::new();
    // Prefer explicitly known font files so text rendering is reliable in containers.
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
    fontdb.load_system_fonts();
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
    let font_size = ((height as f32 * 0.020) as u32).max(13);
    let padding = 16u32;
    let text_x = width / 2;
    let text_y = height.saturating_sub(padding);

    let nodes = format!(
        "<text x=\"{text_x}\" y=\"{text_y}\" font-family=\"Geist Pixel, DejaVu Sans Mono, DejaVu Sans, sans-serif\" font-size=\"{font_size}\" fill=\"rgb(0,0,0)\" text-anchor=\"middle\">created with rideviz.online</text>"
    );

    svg.replacen("</svg>", &format!("{nodes}</svg>"), 1)
}
