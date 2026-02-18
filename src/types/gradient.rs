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
}
