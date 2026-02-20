use std::{f64::consts::PI, io::Cursor};

use apng::{create_config, image_png, Encoder, Frame, PNGImage};
use rayon::prelude::*;

use crate::error::RasterError;
use crate::pipeline::{rasterize, render};
use crate::types::viz::{AnimationEasing, OutputConfig, RenderOptions, VizData};

pub fn render_apng(
    data: &VizData,
    options: &RenderOptions,
    output: &OutputConfig,
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
            let progress = eased_progress(linear_progress, options.animation_easing);

            let svg = render::render_svg_frame(data, options, progress).map_err(|err| {
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

fn eased_progress(t: f64, easing: AnimationEasing) -> f64 {
    match easing {
        AnimationEasing::EaseInOutSine => ease_in_out_sine(t),
    }
}

fn ease_in_out_sine(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    0.5 * (1.0 - (PI * t).cos())
}



