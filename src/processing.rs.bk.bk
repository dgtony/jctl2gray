use std::collections::HashMap;
use std::io::{self, BufRead, BufReader};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::process;
use std::time::SystemTime;

use regex::Regex;
use serde_json;

use errors::{Error, Result};

use config::Config;
use gelf::{ChunkSize, ChunkedMessage, Message, OptFieldsIterator, WireMessage};
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

pub fn process_journalctl(config: Config) -> Result<()> {
    // check OS
    if !is_platform_supported() {
        return Err(Error::InternalError(format!(
            "operating system currently unsupported"
        )));
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

    // bind to socket
    let sender = create_sender_udp(config.sender_port)?;

    // obtain target address (first resolve may fail)
    let (mut target_addr, mut target_addr_updated_at) = get_target_addr(&config.graylog_addr)?;

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

            // renew outdated target address
            if target_addr_updated_at
                .elapsed()
                .unwrap_or_default()
                .as_secs() > config.graylog_addr_ttl
            {
                match get_target_addr(&config.graylog_addr) {
                    Ok((addr, updated_at)) => {
                        target_addr = addr;
                        target_addr_updated_at = updated_at;
                        debug!("target address updated");
                    }

                    // use outdated address
                    Err(e) => warn!("cannot resolve graylog address: {}", e),
                }
            }

            process_log_record(msg, &config, &sender, &target_addr);
        }

        buff.clear();
    }
}

pub fn process_stdin(config: Config) -> Result<()> {
    // bind to socket
    let sender = create_sender_udp(config.sender_port)?;

    // obtain target address (first resolve may fail)
    let (mut target_addr, mut target_addr_updated_at) = get_target_addr(&config.graylog_addr)?;

    debug!("start reading from stdin");

    let stdin_stream = io::stdin();
    for raw in stdin_stream.lock().lines() {
        // renew outdated target address
        if target_addr_updated_at
            .elapsed()
            .unwrap_or_default()
            .as_secs() > config.graylog_addr_ttl
        {
            match get_target_addr(&config.graylog_addr) {
                Ok((addr, updated_at)) => {
                    target_addr = addr;
                    target_addr_updated_at = updated_at;
                    debug!("target address updated");
                }

                // use outdated address
                Err(e) => warn!("cannot resolve graylog address: {}", e),
            }
        }

        match raw {
            Ok(log_line) => {
                process_log_record(&log_line.trim(), &config, &sender, &target_addr);
            }

            Err(err) => return Err(Error::from(err)),
        }
    }

    Ok(())
}

fn process_log_record(data: &str, config: &Config, sender: &UdpSocket, target: &SocketAddr) {
    match transform_record(data, config) {
        Ok(compressed_gelf) => {
            if let Some(chunked) = ChunkedMessage::new(ChunkSize::WAN, compressed_gelf) {
                for chunk in chunked.iter() {
                    if let Err(e) = sender.send_to(&chunk, &target) {
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
fn transform_record(data: &str, config: &Config) -> Result<Vec<u8>> {
    // decode
    let decoded: LogRecord = serde_json::from_str(data)?;

    // absolutely mandatory field
    let short_msg = decoded
        .get("MESSAGE")
        .ok_or(Error::NoMessage)?
        .to_owned()
        .to_string();

    let host = decoded
        .get("_HOSTNAME")
        .map_or("undefined".to_string(), |h| h.to_string());

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
        ts.as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .and_then(|t| Some(msg.set_timestamp(t / 1_000_000 as f64)));
    }

    // additional fields
    for (k, v) in decoded.into_iter() {
        if is_metadata(&k) {
            msg.set_metadata(k, v);
        }
    }

    config.compression.compress(&WireMessage::new(
        msg,
        OptFieldsIterator::new(&config.optional),
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

/// Just bind a socket to any interface.
fn create_sender_udp(port: u16) -> Result<UdpSocket> {
    Ok(UdpSocket::bind(format!("0.0.0.0:{}", port))?)
}

/// Try to resolve and return first IP-address for given host.
fn get_target_addr(host: &str) -> io::Result<(SocketAddr, SystemTime)> {
    let mut addrs = host.to_socket_addrs()?;

    // UDP sendto always takes first resolved address
    let target_addr = addrs
        .next()
        .ok_or(io::Error::new(io::ErrorKind::Other, "empty address list"))?;
    let target_addr_updated_at = SystemTime::now();

    Ok((target_addr, target_addr_updated_at))
}
