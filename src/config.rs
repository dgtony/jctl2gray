use std::io::Read;
use std::fs::File;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use toml;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

use errors::{Error, Result};
use gelf::{LevelMsg, LevelSystem, MessageCompression};

pub type SharedConfig = Arc<Mutex<Config>>;
pub type SharedFlag = Arc<AtomicBool>;

#[derive(Debug, Copy, Clone, Deserialize)]
pub enum LogSource {
    #[serde(rename = "stdin")]
    Stdin,
    #[serde(rename = "journal")]
    Journalctl,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigGlobal {
    pub log_source: LogSource,
    pub sender_port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigWatched {
    pub graylog_addr: String,
    pub compression: MessageCompression,
    pub team: Option<String>,
    pub service: Option<String>,
    pub log_level_system: LevelSystem,
    pub log_level_message: Option<LevelMsg>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub global: ConfigGlobal,
    pub watched: ConfigWatched,
}

// Consume new config, updating current
pub fn update_current(current_config: &mut ConfigWatched, new_config: ConfigWatched) {
    // update some fields in current_config and log it
    if new_config.graylog_addr != current_config.graylog_addr {
        if new_config.graylog_addr.len() > 2 {
            info!("graylog address changed to {}", new_config.graylog_addr);
            current_config.graylog_addr = new_config.graylog_addr;
        } else {
            warn!(
                "bad graylog address in configuration: {}",
                new_config.graylog_addr
            );
        }
    }

    if new_config.log_level_system != current_config.log_level_system {
        info!(
            "system logging level changed to {}",
            new_config.log_level_system
        );
        current_config.log_level_system = new_config.log_level_system;
    }

    if new_config.log_level_message != current_config.log_level_message {
        let new_msg_level = match new_config.log_level_message {
            Some(level) => level.to_string(),
            None => "undefined".to_string(),
        };
        info!("message logging level changed to {}", new_msg_level);
        current_config.log_level_message = new_config.log_level_message;
    }

    if new_config.compression != current_config.compression {
        info!("compression method changed to {}", new_config.compression);
        current_config.compression = new_config.compression;
    }

    if new_config.team != current_config.team {
        info!(
            "team changed to {}",
            new_config.team.as_ref().map_or("undefined", |s| s.as_str())
        );
        current_config.team = new_config.team;
    }

    if new_config.service != current_config.service {
        info!(
            "service name changed to {}",
            new_config.service.as_ref().map_or(
                "undefined",
                |s| s.as_str(),
            )
        );
        current_config.service = new_config.service;
    }
}

// read and parse config
pub fn read_config(config_path: &str) -> Result<Config> {
    // read config from file
    let mut config_buf = String::new();

    debug!("reading configuration from {}", config_path);

    File::open(config_path)?.read_to_string(&mut config_buf)?;
    let config: Config = toml::from_str(&config_buf)?;

    Ok(config)
}

pub fn watch_config(
    config_path: &str,
    config: SharedConfig,
    change_flag: SharedFlag,
) -> Result<()> {
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))?;

    // watch only specified file
    watcher.watch(config_path, RecursiveMode::NonRecursive)?;

    loop {
        match rx.recv() {
            Ok(DebouncedEvent::Write(_)) => {
                debug!("config change detected");
                let new_config = read_config(config_path)?;

                {
                    // update config
                    let mut config = config.lock().unwrap();
                    *config = new_config;
                }

                // finally notify threads
                change_flag.store(true, Ordering::Relaxed);
            }

            Ok(DebouncedEvent::Remove(_)) => warn!("config file was moved or deleted"),

            Err(err) => {
                return Err(Error::InternalError(format!("config watching: {}", err)));
            }

            // no action for other events
            _ => {}
        }
    }
}
