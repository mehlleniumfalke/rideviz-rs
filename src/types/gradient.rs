use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gradient {
    pub name: &'static str,
    pub colors: [&'static str; 2],
}

impl Gradient {
    pub fn get(name: &str) -> Option<Self> {
        match name {
            "fire" => Some(Self {
                name: "fire",
                colors: ["#FF3366", "#FF9933"],
            }),
            "ocean" => Some(Self {
                name: "ocean",
                colors: ["#0055FF", "#00D1FF"],
            }),
            "sunset" => Some(Self {
                name: "sunset",
                colors: ["#FF7E5F", "#FEB47B"],
            }),
            "forest" => Some(Self {
                name: "forest",
                colors: ["#1D976C", "#93F9B9"],
            }),
            "violet" => Some(Self {
                name: "violet",
                colors: ["#8E2DE2", "#4A00E0"],
            }),
            "rideviz" => Some(Self {
                name: "rideviz",
                colors: ["#00C2FF", "#00FF94"],
            }),
            "white" => Some(Self {
                name: "white",
                colors: ["#FFFFFF", "#FFFFFF"],
            }),
            "black" => Some(Self {
                name: "black",
                colors: ["#000000", "#000000"],
            }),
            _ => None,
        }
    }

    pub fn default() -> Self {
        Self {
            name: "fire",
            colors: ["#FF3366", "#FF9933"],
        }
    }

    pub fn interpolate(&self, t: f64) -> String {
        let t = t.clamp(0.0, 1.0);
        let start = parse_hex_color(self.colors[0]).unwrap_or((255, 255, 255));
        let end = parse_hex_color(self.colors[1]).unwrap_or((255, 255, 255));
        let r = lerp_u8(start.0, end.0, t);
        let g = lerp_u8(start.1, end.1, t);
        let b = lerp_u8(start.2, end.2, t);
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
