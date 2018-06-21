use serde;
use serde::ser::SerializeMap;
use serde_json;

use std::time::{SystemTime, UNIX_EPOCH};

use super::{ChunkSize, ChunkedMessage, Message, MessageCompression};
use errors::{Error, Result};

const GELF_VERSION: &str = "1.1";

/// WireMessage is the representation of a fully assembled GELF message
///
/// A WireMessage can be serialized to GELF/JSON (with and without compression)
/// and is the abstraction passed to the transportation backends.
pub struct WireMessage<'a> {
    message: Message<'a>,
    optional: OptFieldsIterator<'a>,
}

impl<'a> WireMessage<'a> {
    /// Construct a new wire message
    ///
    /// The logger is required for populating the `host`-field and metadata
    /// fields which were not added to the message.
    pub fn new(msg: Message<'a>, optional: OptFieldsIterator<'a>) -> Self {
        WireMessage {
            message: msg,
            optional,
        }
    }

    /// Return a GELF/JSON string of this message
    pub fn to_gelf(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Return a compressed GELF/JSON string of this message
    pub fn to_compressed_gelf(&self, compression: MessageCompression) -> Result<Vec<u8>> {
        compression.compress(&self)
    }

    /// Serialize the messages and prepare it for chunking
    pub fn to_chunked_message(
        &self,
        chunk_size: ChunkSize,
        compression: MessageCompression,
    ) -> Result<ChunkedMessage> {
        let msg = self.to_compressed_gelf(compression)?;
        ChunkedMessage::new(chunk_size, msg).ok_or(Error::InternalError(format!(
            "failed to split message on {}-bytes chunks",
            chunk_size.size()
        )))
    }
}

impl<'a> serde::Serialize for WireMessage<'a> {
    /// Serialize the message to a GELF/JSON string
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        let trimmed_symbols: &[_] = &['"', ' '];

        map.serialize_entry("version", GELF_VERSION)?;

        map.serialize_entry("host", self.message.host.trim_matches(trimmed_symbols))?;

        map.serialize_entry(
            "short_message",
            self.message.short_message().trim_matches(trimmed_symbols),
        )?;

        let level = self.message.level as u8;
        map.serialize_entry("level", &level)?;

        if self.message.full_message().is_some() {
            map.serialize_entry("full_message", &self.message.full_message())?;
        }

        map.serialize_key("timestamp")?;
        if self.message.timestamp().is_some() {
            map.serialize_value(&self.message.timestamp)?;
        } else {
            map.serialize_value(&current_time_unix())?;
        }

        for (k, v) in self.optional.clone().into_iter() {
            map.serialize_entry(k, v)?;
        }

        for (key, value) in self.message.all_metadata().iter() {
            let key = "_".to_string() + key;
            map.serialize_entry(&key, value)?;
        }

        map.end()
    }
}

#[derive(Clone)]
pub struct OptFieldsIterator<'a> {
    fields: &'a Vec<(String, String)>,
    position: usize,
}

impl<'a> OptFieldsIterator<'a> {
    pub fn new(fields: &'a Vec<(String, String)>) -> Self {
        OptFieldsIterator {
            fields,
            position: 0,
        }
    }
}

impl<'a> Iterator for OptFieldsIterator<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        let (ref f, ref v) = self.fields.get(self.position)?;
        self.position += 1;
        Some((f, v))
    }
}

/// Return current UNIX-timestamp as a seconds
#[inline]
fn current_time_unix() -> f64 {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock failed");
    ts.as_secs() as f64 + ts.subsec_nanos() as f64 / 1_000_000_000 as f64
}
