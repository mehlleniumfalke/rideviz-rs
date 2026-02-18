mod fit;
mod gpx;

use crate::error::ParseError;
use crate::types::activity::{FileFormat, ParsedActivity};

pub trait Parser {
    fn parse(&self, bytes: &[u8]) -> Result<ParsedActivity, ParseError>;
}

pub fn parse(bytes: &[u8], format: FileFormat) -> Result<ParsedActivity, ParseError> {
    match format {
        FileFormat::Gpx => gpx::GpxParser.parse(bytes),
        FileFormat::Fit => fit::FitParser.parse(bytes),
    }
}
