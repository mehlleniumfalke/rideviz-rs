use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gradient {
    pub name: &'static str,
    pub colors: Vec<&'static str>,
}

impl Gradient {
    pub fn get(name: &str) -> Option<Self> {
        match name {
            "fire" => Some(Self {
                name: "fire",
                colors: vec!["#FF3366", "#FF6600", "#FF9933"],
            }),
            "ocean" => Some(Self {
                name: "ocean",
                colors: vec!["#0055FF", "#0099DD", "#00D1FF"],
            }),
            "sunset" => Some(Self {
                name: "sunset",
                colors: vec!["#FF2D55", "#FF7E5F", "#FEB47B"],
            }),
            "forest" => Some(Self {
                name: "forest",
                colors: vec!["#1D976C", "#4CD964", "#93F9B9"],
            }),
            "violet" => Some(Self {
                name: "violet",
                colors: vec!["#FF0080", "#8E2DE2", "#4A00E0"],
            }),
            "rideviz" => Some(Self {
                name: "rideviz",
                colors: vec!["#00C2FF", "#00EABD", "#00FF94"],
            }),
            "white" => Some(Self {
                name: "white",
                colors: vec!["#FFFFFF", "#FFFFFF", "#FFFFFF"],
            }),
            "black" => Some(Self {
                name: "black",
                colors: vec!["#000000", "#000000", "#000000"],
            }),
            _ => None,
        }
    }

    pub fn default() -> Self {
        Self {
            name: "fire",
            colors: vec!["#FF3366", "#FF6600", "#FF9933"],
        }
    }

    pub fn interpolate(&self, t: f64) -> String {
        let t = t.clamp(0.0, 1.0);
        let stops = &self.colors;
        if stops.is_empty() {
            return "#FFFFFF".to_string();
        }
        if stops.len() == 1 {
            return stops[0].to_string();
        }
        let segments = (stops.len() - 1) as f64;
        let scaled = t * segments;
        let idx = (scaled.floor() as usize).min(stops.len() - 2);
        let local_t = scaled - idx as f64;
        let start = parse_hex_color(stops[idx]).unwrap_or((255, 255, 255));
        let end = parse_hex_color(stops[idx + 1]).unwrap_or((255, 255, 255));
        let r = lerp_u8(start.0, end.0, local_t);
        let g = lerp_u8(start.1, end.1, local_t);
        let b = lerp_u8(start.2, end.2, local_t);
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }
}

fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let value = hex.trim_start_matches('#');
    if value.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&value[0..2], 16).ok()?;
    let g = u8::from_str_radix(&value[2..4], 16).ok()?;
    let b = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some((r, g, b))
}

fn lerp_u8(start: u8, end: u8, t: f64) -> u8 {
    let value = start as f64 + (end as f64 - start as f64) * t;
    value.round().clamp(0.0, 255.0) as u8
}
