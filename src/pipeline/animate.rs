use std::io::Cursor;

use apng::{create_config, image_png, Encoder, Frame, PNGImage};
use rayon::prelude::*;

use crate::error::RasterError;
use crate::pipeline::{rasterize, render};
use crate::types::viz::{OutputConfig, RenderOptions, StatOverlayItem, VizData};

pub fn render_apng(
    data: &VizData,
    options: &RenderOptions,
    output: &OutputConfig,
    stats: &[StatOverlayItem],
) -> Result<Vec<u8>, RasterError> {
    let frame_count = options.animation_frames.max(8);
    let frames: Vec<PNGImage> = (0..frame_count)
        .into_par_iter()
        .map(|idx| {
            let linear_progress = if frame_count <= 1 {
                1.0
            } else {
                idx as f64 / (frame_count - 1) as f64
            };
            let progress = map_linear_progress_to_route(data, linear_progress);

            let svg = render::render_svg_frame(data, options, progress, stats).map_err(|err| {
                RasterError::AnimationFailed(format!(
                    "Failed to render animation frame {}: {}",
                    idx, err
                ))
            })?;

            let png_bytes = rasterize::rasterize(&svg, output)?;
            png_image_from_bytes(&png_bytes, idx)
        })
        .collect::<Result<Vec<_>, RasterError>>()?;

    let config = create_config(&frames, None)
        .map_err(|err| RasterError::AnimationFailed(format!("Failed to build APNG config: {}", err)))?;

    let mut output_bytes = Vec::new();
    {
        let mut cursor = Cursor::new(&mut output_bytes);
        let mut encoder = Encoder::new(&mut cursor, config)
            .map_err(|err| RasterError::AnimationFailed(format!("Failed to create APNG encoder: {}", err)))?;
        let delay_ms = (options.animation_duration_ms / frame_count.max(1)).max(16);
        let frame = Frame {
            delay_num: Some(delay_ms.min(u16::MAX as u32) as u16),
            delay_den: Some(1000),
            ..Default::default()
        };
        encoder.encode_all(frames, Some(&frame)).map_err(|err| {
            RasterError::AnimationFailed(format!("Failed to encode APNG frames: {}", err))
        })?;
    }

    Ok(output_bytes)
}

pub fn map_linear_progress_to_route(data: &VizData, linear_progress: f64) -> f64 {
    let linear_progress = linear_progress.clamp(0.0, 1.0);
    if data.points.len() < 2 {
        return linear_progress;
    }

    let mut first_sample: Option<(f64, f64)> = None;
    let mut last_sample: Option<(f64, f64)> = None;
    for point in &data.points {
        if let Some(elapsed) = point.elapsed_seconds {
            let sample = (elapsed, point.route_progress);
            if first_sample.is_none() {
                first_sample = Some(sample);
            }
            last_sample = Some(sample);
        }
    }

    let Some((first_elapsed, first_progress)) = first_sample else {
        return linear_progress;
    };
    let Some((total_elapsed, _)) = last_sample else {
        return linear_progress;
    };

    if total_elapsed <= f64::EPSILON {
        return linear_progress;
    }

    let target_elapsed = linear_progress * total_elapsed;
    if target_elapsed <= first_elapsed {
        return first_progress.clamp(0.0, 1.0);
    }

    let mut prev_sample: Option<(f64, f64)> = None;
    for point in &data.points {
        let Some(curr_elapsed) = point.elapsed_seconds else {
            continue;
        };
        let curr_progress = point.route_progress;
        if let Some((prev_elapsed, prev_progress)) = prev_sample {
            if curr_elapsed <= prev_elapsed {
                prev_sample = Some((curr_elapsed, curr_progress));
                continue;
            }
            if target_elapsed <= curr_elapsed {
                let local_t = ((target_elapsed - prev_elapsed) / (curr_elapsed - prev_elapsed))
                    .clamp(0.0, 1.0);
                return (prev_progress + (curr_progress - prev_progress) * local_t).clamp(0.0, 1.0);
            }
        }
        prev_sample = Some((curr_elapsed, curr_progress));
    }

    last_sample
        .map(|(_, progress)| progress.clamp(0.0, 1.0))
        .unwrap_or(linear_progress)
}

fn png_image_from_bytes(png_bytes: &[u8], frame_idx: u32) -> Result<PNGImage, RasterError> {
    let decoder = image_png::Decoder::new(Cursor::new(png_bytes));
    let mut reader = decoder.read_info().map_err(|err| {
        RasterError::AnimationFailed(format!(
            "Failed to decode PNG metadata for frame {}: {}",
            frame_idx, err
        ))
    })?;
    let mut data = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut data).map_err(|err| {
        RasterError::AnimationFailed(format!(
            "Failed to decode PNG pixels for frame {}: {}",
            frame_idx, err
        ))
    })?;
    data.truncate(info.buffer_size());

    Ok(PNGImage {
        width: info.width,
        height: info.height,
        data,
        color_type: info.color_type,
        bit_depth: info.bit_depth,
    })
}



