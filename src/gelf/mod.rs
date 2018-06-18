mod chunked_message;
mod compression;
mod level;
mod wire_message;

pub use self::chunked_message::{ChunkSize, ChunkedMessage};
pub use self::compression::MessageCompression;
pub use self::level::{LevelMsg, LevelSystem};
pub use self::wire_message::WireMessage;

use serde_json::Value;
use std::collections::HashMap;

/// Message is the representation of a GELF message.
///
/// `Message` provides a fluid setter and getter interface to all of GELF's
/// features.
pub struct Message<'a> {
    host: &'a str,
    short_message: String,
    full_message: Option<String>,
    timestamp: Option<f64>,
    level: LevelSystem,

    metadata: HashMap<String, Value>,
}

impl<'a> Message<'a> {
    /// Construct a new GELF-message.
    ///
    /// All fields will use their defaults. This means usually Option::None.
    /// A notable exception is `level`. The GELF spec requires this field to
    /// default to Alert.
    pub fn new(host: &'a str, short_message: String) -> Self {
        Message {
            host,
            short_message,
            full_message: None,
            timestamp: None, // if not set - will be added during serialization
            level: LevelSystem::Alert,
            metadata: HashMap::new(),
        }
    }

    /// Return the `short_message`
    pub fn short_message(&self) -> &str {
        &self.short_message
    }

    /// Set the `short_message`
    pub fn set_short_message(&mut self, msg: String) -> &mut Self {
        self.short_message = msg;
        self
    }

    /// Return the `full_message`
    pub fn full_message(&self) -> &Option<String> {
        &self.full_message
    }

    /// Set the `full_message`
    pub fn set_full_message(&mut self, msg: String) -> &mut Self {
        self.full_message = Some(msg);
        self
    }

    // Clear the `full_message`
    pub fn clear_full_message(&mut self) -> &mut Self {
        self.full_message = None;
        self
    }

    /// Return the `timestamp`
    pub fn timestamp(&self) -> &Option<f64> {
        &self.timestamp
    }

    /// Set the `timestamp`
    pub fn set_timestamp(&mut self, ts: f64) -> &mut Self {
        self.timestamp = Some(ts);
        self
    }

    /// Clear the `timestamp`
    pub fn clear_timestamp(&mut self) -> &mut Self {
        self.timestamp = None;
        self
    }

    /// Return the `level`
    pub fn level(&self) -> LevelSystem {
        self.level
    }

    /// Set the `level`
    pub fn set_level(&mut self, level: LevelSystem) -> &mut Self {
        self.level = level;
        self
    }

    /// Return a metadata field with given key
    pub fn metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    /// Return all metadata
    //pub fn all_metadata(&self) -> &HashMap<&'a str, String> {
    pub fn all_metadata(&self) -> &HashMap<String, Value> {
        &self.metadata
    }

    /// Set a metadata field with given key to value
    pub fn set_metadata(&mut self, key: String, value: Value) -> Option<&mut Self> {
        if key == "id" {
            // prohibited ?
            return None;
        }

        self.metadata.insert(key, value);
        Some(self)
    }
}
