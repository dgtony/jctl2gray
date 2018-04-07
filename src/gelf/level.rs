use std::fmt;

/// GELF's representation of an error level
///
/// GELF's error levels are equivalent to syslog's severity
/// information (specified in [RFC 5424](https://tools.ietf.org/html/rfc5424))
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LevelSystem {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Informational,
    Debug,
}

impl LevelSystem {
    /// Get the GELF error level from syslog
    pub fn from_num(level: u8) -> LevelSystem {
        match level {
            0 => LevelSystem::Emergency,
            1 => LevelSystem::Alert,
            2 => LevelSystem::Critical,
            3 => LevelSystem::Error,
            4 => LevelSystem::Warning,
            5 => LevelSystem::Notice,
            6 => LevelSystem::Informational,
            _ => LevelSystem::Debug,
        }
    }

    /// Convert GELF error level for syslog
    pub fn to_num(&self) -> u8 {
        match *self {
            LevelSystem::Emergency => 0,
            LevelSystem::Alert => 1,
            LevelSystem::Critical => 2,
            LevelSystem::Error => 3,
            LevelSystem::Warning => 4,
            LevelSystem::Notice => 5,
            LevelSystem::Informational => 6,
            LevelSystem::Debug => 7,
        }
    }
}

impl Into<u8> for LevelSystem {
    fn into(self) -> u8 {
        self.to_num()
    }
}

impl From<u8> for LevelSystem {
    fn from(level: u8) -> Self {
        LevelSystem::from_num(level)
    }
}

impl fmt::Display for LevelSystem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LevelSystem::Emergency => write!(f, "emergency"),
            LevelSystem::Alert => write!(f, "alert"),
            LevelSystem::Critical => write!(f, "critical"),
            LevelSystem::Error => write!(f, "error"),
            LevelSystem::Warning => write!(f, "warning"),
            LevelSystem::Notice => write!(f, "notice"),
            LevelSystem::Informational => write!(f, "info"),
            LevelSystem::Debug => write!(f, "debug"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LevelMsg {
    Fatal,
    Panic,
    Error,
    Warning,
    Info,
    Debug,
}

impl<'a> From<&'a str> for LevelMsg {
    fn from(level: &'a str) -> Self {
        match level.as_ref() {
            "fatal" => LevelMsg::Fatal,
            "panic" => LevelMsg::Panic,
            "error" => LevelMsg::Error,
            "warning" => LevelMsg::Warning,
            "info" => LevelMsg::Info,
            _ => LevelMsg::Debug,
        }
    }
}

impl fmt::Display for LevelMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LevelMsg::Fatal => write!(f, "fatal"),
            LevelMsg::Panic => write!(f, "panic"),
            LevelMsg::Error => write!(f, "error"),
            LevelMsg::Warning => write!(f, "warning"),
            LevelMsg::Info => write!(f, "info"),
            LevelMsg::Debug => write!(f, "debug"),
        }
    }
}
