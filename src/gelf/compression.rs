use std::fmt;
use std::io;

use libflate::gzip;
use libflate::zlib;

use super::wire_message::WireMessage;
use errors::Result;

/// MessageCompression represents all possible compression algorithms in GELF.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MessageCompression {
    None,
    Gzip,
    Zlib,
}

impl<'a> From<&'a str> for MessageCompression {
    fn from(algorithm: &'a str) -> Self {
        match algorithm {
            "gzip" => MessageCompression::Gzip,
            "zlib" => MessageCompression::Zlib,
            _ => MessageCompression::None,
        }
    }
}

impl MessageCompression {
    /// Return the default compression algorithm.
    pub fn default() -> MessageCompression {
        MessageCompression::Gzip
    }

    /// Compress a serialized message with the defined algorithm.
    pub fn compress(&self, message: &WireMessage) -> Result<Vec<u8>> {
        let json = message.to_gelf()?;

        let compressed = match *self {
            MessageCompression::None => json.into_bytes(),

            MessageCompression::Gzip => {
                let mut cursor = io::Cursor::new(json);
                let mut encoder = gzip::Encoder::new(Vec::new())?;
                io::copy(&mut cursor, &mut encoder)?;
                let encoded = encoder.finish().into_result()?;
                encoded
            }

            MessageCompression::Zlib => {
                let mut cursor = io::Cursor::new(json);
                let mut encoder = zlib::Encoder::new(Vec::new())?;
                io::copy(&mut cursor, &mut encoder)?;
                let encoded = encoder.finish().into_result()?;
                encoded
            }
        };

        Ok(compressed)
    }
}

impl fmt::Display for MessageCompression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MessageCompression::None => write!(f, "none"),
            MessageCompression::Gzip => write!(f, "gzip"),
            MessageCompression::Zlib => write!(f, "zlib"),
        }
    }
}
