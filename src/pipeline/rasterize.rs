use crate::error::RasterError;
use crate::types::viz::OutputConfig;

pub fn rasterize(svg: &str, config: &OutputConfig) -> Result<Vec<u8>, RasterError> {
    let options = usvg::Options::default();
    let fontdb = usvg::fontdb::Database::new();
    let tree = usvg::Tree::from_str(svg, &options, &fontdb)
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
