use std::net::UdpSocket;
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader};
use std::sync::atomic::Ordering;
use std::process;
use serde_json;
use regex::Regex;

use errors::{Error, Result};

use config::{update_current, Config, ConfigWatched, SharedConfig, SharedFlag};
use gelf::{ChunkSize, ChunkedMessage, Message, WireMessage};
use gelf::{LevelMsg, LevelSystem};

const IGNORED_FIELDS: [&str; 9] = [
    "MESSAGE",
    "_HOSTNAME",
    "__REALTIME_TIMESTAMP",
    "PRIORITY",
    "__CURSOR",
    "_BOOT_ID",
    "_MACHINE_ID",
    "_SYSTEMD_CGROUP",
    "_SYSTEMD_SLICE",
];

type LogRecord = HashMap<String, serde_json::Value>;

pub fn process_journalctl(config: SharedConfig, config_changed: SharedFlag) -> Result<()> {
    // check OS
    if !is_platform_supported() {
        return Err(Error::InternalError(
            format!("operating system currently unsupported"),
        ));
    }

    let mut subprocess = process::Command::new("journalctl")
        .args(&["-o", "json", "-f"])
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()?;

    // Dirty trick. In theory it doesn't have to work, because an operating system
    // is allowed to make the BufReader wait for more data in read, but in practice
    // the operating systems prefer the early "short reads" to waiting.
    let mut subprocess_stdout = BufReader::new(subprocess.stdout.as_mut().unwrap());
    let mut subprocess_stderr = BufReader::new(subprocess.stderr.as_mut().unwrap());
    let mut current_config = config.lock().unwrap().clone();

    // bind to socket
    let sender = create_sender_udp(current_config.global.sender_port)?;

    debug!("start reading from journalctl");

    let mut buff = String::new();

    loop {
        subprocess_stdout.read_line(&mut buff)?;

        {
            let msg = buff.trim();

            // verify if stdout was closed
            if msg.len() < 1 {
                let mut err_buff = String::new();
                subprocess_stderr.read_line(&mut err_buff)?;
                return Err(Error::InternalError(err_buff));
            }

            if config_changed.load(Ordering::Relaxed) {
                // reload config
                let new_config = config.lock().unwrap().clone();
                update_current(&mut current_config.watched, new_config.watched);

                // reset flag
                config_changed.store(false, Ordering::Relaxed);
            }

            process_log_record(msg, &current_config, &sender);
        }

        buff.clear();
    }
}

pub fn process_stdin(config: SharedConfig, config_changed: SharedFlag) -> Result<()> {
    // local copy
    let mut current_config = config.lock().unwrap().clone();

    // bind to socket
    let sender = create_sender_udp(current_config.global.sender_port)?;

    debug!("start reading from stdin");

    let stdin_stream = io::stdin();
    for raw in stdin_stream.lock().lines() {
        if config_changed.load(Ordering::Relaxed) {
            // reload config
            let new_config = config.lock().unwrap().clone();
            update_current(&mut current_config.watched, new_config.watched);

            // reset flag
            config_changed.store(false, Ordering::Relaxed);
        }

        match raw {
            Ok(log_line) => {
                process_log_record(&log_line.trim(), &current_config, &sender);
            }

            Err(err) => return Err(Error::from(err)),
        }
    }

    Ok(())
}

fn process_log_record(data: &str, config: &Config, sender: &UdpSocket) {
    match transform_record(data, &config.watched) {
        Ok(compressed_gelf) => {
            if let Some(chunked) = ChunkedMessage::new(ChunkSize::WAN, compressed_gelf) {
                for chunk in chunked.iter() {
                    if let Err(e) = sender.send_to(&chunk, &config.watched.graylog_addr) {
                        error!("sender failure: {}", e);
                    }
                }
            }
        }

        // ignore
        Err(Error::InsufficientLogLevel) => {}

        Err(Error::NoMessage) => debug!("no message field found"),

        Err(e) => warn!("parsing error: {}, message: {}", e, data),
    }
}

/// Try to decode original JSON, transform fields to GELF format, serialize and compress it.
fn transform_record(data: &str, config: &ConfigWatched) -> Result<Vec<u8>> {
    // decode
    let decoded: LogRecord = serde_json::from_str(data)?;

    // absolutely mandatory field
    let short_msg = decoded
        .get("MESSAGE")
        .ok_or(Error::NoMessage)?
        .to_owned()
        .to_string();

    let host = decoded.get("_HOSTNAME").map_or(
        "undefined".to_string(),
        |h| h.to_string(),
    );

    // filter by message level
    if config.log_level_message.is_some() {
        if let Some(msg_level) = get_msg_log_level(&short_msg) {
            if msg_level > config.log_level_message.unwrap() {
                return Err(Error::InsufficientLogLevel);
            }
        }
    }

    // create GELF-message
    let mut msg = Message::new(&host, short_msg);

    // filter by system log-level
    if let Some(log_level) = decoded
        .get("PRIORITY")
        .and_then(|raw_level| raw_level.as_str())
        .and_then(|value| value.parse::<u8>().ok())
        .and_then(|num_level| Some(LevelSystem::from(num_level)))
    {
        if log_level > config.log_level_system {
            return Err(Error::InsufficientLogLevel);
        }

        msg.set_level(log_level);
    }

    // timestamp
    if let Some(ts) = decoded.get("__REALTIME_TIMESTAMP") {
        // convert from systemd's format of microseconds expressed as
        // an integer to graylog's float format, eg: "seconds.microseconds"
        ts.as_f64().and_then(
            |t| Some(msg.set_timestamp(t / 1_000_000 as f64)),
        );
    }

    // additional fields
    for (k, v) in decoded.into_iter() {
        if is_metadata(&k) {
            msg.set_metadata(k, v);
        }
    }

    // serialize and compress
    config.compression.compress(&WireMessage::new(
        msg,
        config.team.as_ref().map(|s| s.as_str()),
        config.service.as_ref().map(|s| s.as_str()),
    ))
}

fn is_metadata(field: &str) -> bool {
    for &f in IGNORED_FIELDS.iter() {
        if f == field {
            return false;
        }
    }

    return true;
}

fn is_platform_supported() -> bool {
    if cfg!(target_os = "linux") {
        true
    } else {
        false
    }
}

fn get_msg_log_level(msg: &str) -> Option<LevelMsg> {
    lazy_static! {
        // try to find pattern in message: 'level=some_log_level'
        static ref RE: Regex = Regex::new(r#"level=([a-zA-Z]+ )"#).unwrap();
    }

    // first group match
    let level = RE.captures(msg)?.get(1)?.as_str().trim();
    Some(LevelMsg::from(level))
}

fn create_sender_udp(port: u16) -> Result<UdpSocket> {
    Ok(UdpSocket::bind(format!("0.0.0.0:{}", port))?)
}
