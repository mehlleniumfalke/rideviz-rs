use std::{f64::consts::PI, io::Cursor};
use tempfile::NamedTempFile;

use apng::{create_config, image_png, Encoder, Frame, PNGImage};
use rayon::prelude::*;

use crate::error::RasterError;
use crate::pipeline::{rasterize, render};
use crate::types::viz::{AnimationEasing, OutputConfig, RenderOptions, VizData};

extern crate ffmpeg_next as ffmpeg;

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

pub fn render_webm(
    data: &VizData,
    options: &RenderOptions,
    output: &OutputConfig,
) -> Result<Vec<u8>, RasterError> {
    use ffmpeg::format::Pixel;
    use ffmpeg::software::scaling::{Context as ScalerCtx, Flags as ScalerFlags};
    use ffmpeg::{codec, encoder, format, frame, Rational};

    ffmpeg::init().map_err(|e| RasterError::AnimationFailed(format!("FFmpeg init failed: {}", e)))?;

    let frame_count = options.animation_frames.max(8);
    let width = output.width;
    let height = output.height;

    // Render all frames as PNG bytes in parallel
    let png_frames: Vec<Vec<u8>> = (0..frame_count)
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

            rasterize::rasterize(&svg, output)
        })
        .collect::<Result<Vec<_>, RasterError>>()?;

    // Decode PNG frames to raw RGBA
    let rgba_frames: Vec<Vec<u8>> = png_frames
        .iter()
        .enumerate()
        .map(|(idx, png_bytes)| decode_png_to_rgba(png_bytes, idx))
        .collect::<Result<Vec<_>, RasterError>>()?;

    // Calculate FPS from duration
    let fps = (frame_count as f64 * 1000.0 / options.animation_duration_ms as f64).round() as i32;
    let fps = fps.max(1);

    // Write to a temp file (FFmpeg requires a path, not an in-memory buffer)
    let tmp = NamedTempFile::new()
        .map_err(|e| RasterError::AnimationFailed(format!("Failed to create temp file: {}", e)))?;
    let tmp_path = tmp.path().to_path_buf();

    {
        // Create output format context for WebM
        let mut octx = format::output_as(&tmp_path, "webm")
            .map_err(|e| RasterError::AnimationFailed(format!("Failed to create WebM output: {}", e)))?;

        // Find VP9 encoder (libvpx-vp9 supports alpha)
        let codec = encoder::find(codec::Id::VP9)
            .ok_or_else(|| RasterError::AnimationFailed("VP9 encoder not found".to_string()))?;

        // Check global header flag before borrowing octx mutably via add_stream
        let needs_global_header = octx.format().flags().contains(format::Flags::GLOBAL_HEADER);

        // Add video stream
        let mut ost = octx.add_stream(codec)
            .map_err(|e| RasterError::AnimationFailed(format!("Failed to add stream: {}", e)))?;

        let stream_index = ost.index();

        // Configure encoder
        let mut encoder_ctx = codec::context::Context::new_with_codec(codec)
            .encoder()
            .video()
            .map_err(|e| RasterError::AnimationFailed(format!("Failed to create encoder: {}", e)))?;

        encoder_ctx.set_width(width);
        encoder_ctx.set_height(height);
        encoder_ctx.set_format(Pixel::YUVA420P); // VP9 with alpha
        encoder_ctx.set_time_base(Rational(1, fps));
        encoder_ctx.set_frame_rate(Some(Rational(fps, 1)));

        if needs_global_header {
            encoder_ctx.set_flags(codec::Flags::GLOBAL_HEADER);
        }

        // Open encoder with options for quality
        let mut encoder_opts = ffmpeg::Dictionary::new();
        encoder_opts.set("crf", "30");
        encoder_opts.set("b:v", "0");
        encoder_opts.set("deadline", "good");
        encoder_opts.set("cpu-used", "4");

        let mut encoder = encoder_ctx
            .open_with(encoder_opts)
            .map_err(|e| RasterError::AnimationFailed(format!("Failed to open encoder: {}", e)))?;

        ost.set_parameters(&encoder);

        // Write header
        octx.write_header()
            .map_err(|e| RasterError::AnimationFailed(format!("Failed to write header: {}", e)))?;

        let ost_time_base = octx.stream(stream_index).unwrap().time_base();

        // Create scaler to convert RGBA to YUVA420P
        let mut scaler = ScalerCtx::get(
            Pixel::RGBA,
            width,
            height,
            Pixel::YUVA420P,
            width,
            height,
            ScalerFlags::BILINEAR,
        )
        .map_err(|e| RasterError::AnimationFailed(format!("Failed to create scaler: {}", e)))?;

        // Encode each frame
        for (idx, rgba_data) in rgba_frames.iter().enumerate() {
            // Create RGBA frame
            let mut rgba_frame = frame::Video::new(Pixel::RGBA, width, height);
            rgba_frame.data_mut(0).copy_from_slice(rgba_data);

            // Convert to YUVA420P
            let mut yuva_frame = frame::Video::empty();
            scaler
                .run(&rgba_frame, &mut yuva_frame)
                .map_err(|e| RasterError::AnimationFailed(format!("Failed to scale frame {}: {}", idx, e)))?;

            yuva_frame.set_pts(Some(idx as i64));

            // Send frame to encoder
            encoder
                .send_frame(&yuva_frame)
                .map_err(|e| RasterError::AnimationFailed(format!("Failed to send frame {}: {}", idx, e)))?;

            // Receive and write packets
            let mut packet = ffmpeg::Packet::empty();
            while encoder.receive_packet(&mut packet).is_ok() {
                packet.set_stream(stream_index);
                packet.rescale_ts(Rational(1, fps), ost_time_base);
                packet
                    .write_interleaved(&mut octx)
                    .map_err(|e| RasterError::AnimationFailed(format!("Failed to write packet: {}", e)))?;
            }
        }

        // Flush encoder
        encoder
            .send_eof()
            .map_err(|e| RasterError::AnimationFailed(format!("Failed to send EOF: {}", e)))?;

        let mut packet = ffmpeg::Packet::empty();
        while encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(stream_index);
            packet.rescale_ts(Rational(1, fps), ost_time_base);
            packet
                .write_interleaved(&mut octx)
                .map_err(|e| RasterError::AnimationFailed(format!("Failed to write final packet: {}", e)))?;
        }

        // Write trailer
        octx.write_trailer()
            .map_err(|e| RasterError::AnimationFailed(format!("Failed to write trailer: {}", e)))?;
    }

    let output_data = std::fs::read(&tmp_path)
        .map_err(|e| RasterError::AnimationFailed(format!("Failed to read temp WebM file: {}", e)))?;

    Ok(output_data)
}

fn decode_png_to_rgba(png_bytes: &[u8], frame_idx: usize) -> Result<Vec<u8>, RasterError> {
    let decoder = png::Decoder::new(Cursor::new(png_bytes));
    let mut reader = decoder.read_info().map_err(|err| {
        RasterError::AnimationFailed(format!(
            "Failed to decode PNG for frame {}: {}",
            frame_idx, err
        ))
    })?;

    let mut data = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut data).map_err(|err| {
        RasterError::AnimationFailed(format!(
            "Failed to read PNG pixels for frame {}: {}",
            frame_idx, err
        ))
    })?;

    Ok(data)
}

