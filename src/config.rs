/// General app config
///
use gelf::{LevelMsg, LevelSystem, MessageCompression};

#[derive(Debug, Copy, Clone, Deserialize)]
pub enum LogSource {
    #[serde(rename = "stdin")]
    Stdin,
    #[serde(rename = "journal")]
    Journalctl,
}

#[derive(Debug)]
pub struct Config {
    pub log_source: LogSource,
    pub sender_port: u16,
    pub graylog_addr: String,
    pub graylog_addr_ttl: u64,
    pub compression: MessageCompression,
    pub team: Option<String>,
    pub service: Option<String>,
    pub log_level_system: LevelSystem,
    pub log_level_message: Option<LevelMsg>,
}

pub fn parse_log_source(level: &str) -> Option<LogSource> {
    match level {
        "stdin" => Some(LogSource::Stdin),
        "journal" => Some(LogSource::Journalctl),
        _ => None,
    }
}
