use crate::error::ParseError;
use crate::pipeline::parse::Parser;
use crate::types::activity::{ParsedActivity, TrackPoint};
use chrono::DateTime;
use fitparser::profile::MesgNum;

pub struct FitParser;

impl Parser for FitParser {
    fn parse(&self, bytes: &[u8]) -> Result<ParsedActivity, ParseError> {
        let data = fitparser::from_bytes(bytes)
            .map_err(|e| ParseError::InvalidFit(format!("Failed to parse FIT file: {}", e)))?;

        let mut points = Vec::new();

        for record in data {
            if record.kind() != MesgNum::Record {
                continue;
            }

            let mut point = TrackPoint {
                lat: 0.0,
                lon: 0.0,
                elevation: None,
                time: None,
                heart_rate: None,
                power: None,
                cadence: None,
                temperature: None,
            };

            let mut has_position = false;

            for field in record.fields() {
                match field.name() {
                    "position_lat" => {
                        if let fitparser::Value::SInt32(val) = field.value() {
                            point.lat = semicircles_to_degrees(*val);
                            has_position = true;
                        }
                    }
                    "position_long" => {
                        if let fitparser::Value::SInt32(val) = field.value() {
                            point.lon = semicircles_to_degrees(*val);
                            has_position = true;
                        }
                    }
                    "altitude" | "enhanced_altitude" => {
                        if let fitparser::Value::Float64(val) = field.value() {
                            point.elevation = Some(*val);
                        }
                    }
                    "timestamp" => {
                        if let fitparser::Value::Timestamp(val) = field.value() {
                            point.time = Some(DateTime::from_timestamp(val.timestamp(), 0).unwrap_or_default());
                        }
                    }
                    "heart_rate" => {
                        if let fitparser::Value::UInt8(val) = field.value() {
                            point.heart_rate = Some(*val as u16);
                        }
                    }
                    "power" => {
                        if let fitparser::Value::UInt16(val) = field.value() {
                            point.power = Some(*val);
                        }
                    }
                    "cadence" => {
                        if let fitparser::Value::UInt8(val) = field.value() {
                            point.cadence = Some(*val as u16);
                        }
                    }
                    "temperature" => {
                        if let fitparser::Value::SInt8(val) = field.value() {
                            point.temperature = Some(*val as f32);
                        }
                    }
                    _ => {}
                }
            }

            if has_position {
                points.push(point);
            }
        }

        if points.is_empty() {
            return Err(ParseError::EmptyFile);
        }

        Ok(ParsedActivity { points })
    }
}

fn semicircles_to_degrees(semicircles: i32) -> f64 {
    (semicircles as f64) * (180.0 / 2_147_483_648.0)
}
