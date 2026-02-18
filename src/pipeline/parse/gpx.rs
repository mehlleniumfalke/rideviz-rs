use crate::error::ParseError;
use crate::pipeline::parse::Parser;
use crate::types::activity::{FileFormat, ParsedActivity, TrackPoint};
use chrono::{DateTime, Utc};
use quick_xml::events::Event;
use quick_xml::Reader;

pub struct GpxParser;

impl Parser for GpxParser {
    fn parse(&self, bytes: &[u8]) -> Result<ParsedActivity, ParseError> {
        let mut reader = Reader::from_reader(bytes);
        reader.trim_text(true);

        let mut points = Vec::new();
        let mut in_trkpt = false;
        let mut current_point: Option<TrackPoint> = None;
        let mut current_element = String::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let name = e.name();
                    let name_str = std::str::from_utf8(name.as_ref())
                        .map_err(|e| ParseError::InvalidGpx(e.to_string()))?;

                    if name_str == "trkpt" {
                        in_trkpt = true;
                        let mut lat = None;
                        let mut lon = None;

                        for attr in e.attributes() {
                            let attr = attr.map_err(|e| ParseError::InvalidGpx(e.to_string()))?;
                            let key = std::str::from_utf8(attr.key.as_ref())
                                .map_err(|e| ParseError::InvalidGpx(e.to_string()))?;
                            let value = std::str::from_utf8(&attr.value)
                                .map_err(|e| ParseError::InvalidGpx(e.to_string()))?;

                            match key {
                                "lat" => lat = value.parse().ok(),
                                "lon" => lon = value.parse().ok(),
                                _ => {}
                            }
                        }

                        if let (Some(lat), Some(lon)) = (lat, lon) {
                            current_point = Some(TrackPoint {
                                lat,
                                lon,
                                elevation: None,
                                time: None,
                                heart_rate: None,
                                power: None,
                                cadence: None,
                                temperature: None,
                            });
                        }
                    } else if in_trkpt {
                        current_element = name_str.to_string();
                    }
                }
                Ok(Event::Text(e)) => {
                    if in_trkpt {
                        if let Some(point) = current_point.as_mut() {
                            let text = e
                                .unescape()
                                .map_err(|e| ParseError::InvalidGpx(e.to_string()))?;

                            match current_element.as_str() {
                                "ele" => point.elevation = text.parse().ok(),
                                "time" => point.time = text.parse::<DateTime<Utc>>().ok(),
                                "hr" | "gpxtpx:hr" => point.heart_rate = text.parse().ok(),
                                "power" | "gpxtpx:power" => point.power = text.parse().ok(),
                                "cad" | "gpxtpx:cad" => point.cadence = text.parse().ok(),
                                "atemp" | "gpxtpx:atemp" => point.temperature = text.parse().ok(),
                                _ => {}
                            }
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let name_str = std::str::from_utf8(name.as_ref())
                        .map_err(|e| ParseError::InvalidGpx(e.to_string()))?;

                    if name_str == "trkpt" {
                        if let Some(point) = current_point.take() {
                            points.push(point);
                        }
                        in_trkpt = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(ParseError::InvalidGpx(e.to_string())),
                _ => {}
            }
            buf.clear();
        }

        if points.is_empty() {
            return Err(ParseError::EmptyFile);
        }

        Ok(ParsedActivity {
            points,
            file_format: FileFormat::Gpx,
        })
    }
}
