use crate::types::viz::VizData;

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

