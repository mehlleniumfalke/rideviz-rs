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
    
    // Load Geist font for watermark
    let geist_bytes = include_bytes!("../../assets/fonts/Geist-Regular.otf");
    fontdb.load_font_data(geist_bytes.to_vec());
    
    fontdb.load_system_fonts();
    fontdb
}

fn rasterize_with_fontdb(
    svg: &str,
    config: &OutputConfig,
    fontdb: &usvg::fontdb::Database,
) -> Result<Vec<u8>, RasterError> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &options, fontdb)
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

    // Apply watermark if enabled
    if config.watermark {
        apply_watermark(&mut pixmap, fontdb)?;
    }

    pixmap
        .encode_png()
        .map_err(|e| RasterError::RenderFailed(format!("Failed to encode PNG: {}", e)))
}

fn apply_watermark(
    pixmap: &mut tiny_skia::Pixmap,
    fontdb: &usvg::fontdb::Database,
) -> Result<(), RasterError> {
    let watermark_text = "rideviz.online";
    let height = pixmap.height() as f32;
    let width = pixmap.width() as f32;
    
    // Font size: 3% of image height
    let font_size = (height * 0.03).max(12.0);
    
    // Padding from edges
    let padding = 20.0;
    
    // Build SVG with text element
    let svg_text = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}">
            <defs>
                <filter id="shadow" x="-50%" y="-50%" width="200%" height="200%">
                    <feGaussianBlur in="SourceAlpha" stdDeviation="2"/>
                    <feOffset dx="0" dy="1" result="offsetblur"/>
                    <feComponentTransfer>
                        <feFuncA type="linear" slope="0.3"/>
                    </feComponentTransfer>
                    <feMerge>
                        <feMergeNode/>
                        <feMergeNode in="SourceGraphic"/>
                    </feMerge>
                </filter>
            </defs>
            <text x="{}" y="{}" font-family="Geist, sans-serif" font-size="{}" 
                  fill="rgba(255, 255, 255, 0.7)" text-anchor="end" filter="url(#shadow)">{}</text>
        </svg>"#,
        width,
        height,
        width - padding,
        height - padding,
        font_size,
        watermark_text
    );

    // Parse and render the watermark SVG
    let options = usvg::Options::default();
    let watermark_tree = usvg::Tree::from_str(&svg_text, &options, fontdb)
        .map_err(|e| RasterError::RenderFailed(format!("Failed to parse watermark SVG: {}", e)))?;

    let transform = tiny_skia::Transform::identity();
    resvg::render(&watermark_tree, transform, &mut pixmap.as_mut());

    Ok(())
}
