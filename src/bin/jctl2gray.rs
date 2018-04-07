#[macro_use]
extern crate log;

extern crate clap;
extern crate jctl2gray;
extern crate loggerv;

use std::process;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

use jctl2gray::config::{read_config, watch_config, LogSource};
use jctl2gray::processing;

fn main() {
    // CLI options
    let args = clap::App::new("journal2graylog")
        .version("0.1")
        .author("Anton Dort-Golts dortgolts@gmail.com")
        .about("Read logs from stdin/journalctl and send it to Graylog")
        .arg(
            clap::Arg::with_name("confpath")
                .short("c")
                .long("config")
                .help("path to configuration file")
                .default_value("config.toml"),
        )
        .get_matches();

    let config_path = args.value_of("confpath").expect("path to config file");

    // init logger
    loggerv::Logger::new()
        .max_level(log_level())
        .level(true)
        .separator(" | ")
        .colors(true)
        .no_module_path()
        .init()
        .unwrap();

    match read_config(config_path) {
        Ok(conf) => {
            let source = conf.global.log_source;

            // shared config
            let config = Arc::new(Mutex::new(conf));

            // conf flag
            let config_changed_flag = Arc::new(AtomicBool::new(false));

            // choose source and start processing input in separate thread
            match source {
                LogSource::Stdin => {
                    let config = config.clone();
                    let change_flag = config_changed_flag.clone();
                    thread::spawn(|| {
                        if let Err(e) = processing::process_stdin(config, change_flag) {
                            error!("stdin processing stopped: {}", e);
                            process::exit(1);
                        }
                    });
                }

                LogSource::Journalctl => {
                    let config = config.clone();
                    let change_flag = config_changed_flag.clone();
                    thread::spawn(|| {
                        if let Err(e) = processing::process_journalctl(config, change_flag) {
                            error!("journalctl processing stopped: {}", e);
                            process::exit(1);
                        }
                    });
                }
            }

            //watch config changes
            if let Err(err) = watch_config(config_path, config, config_changed_flag) {
                error!("config watching: {}", err);
            }
        }

        Err(err) => {
            error!("failed to load configuration: {}", err);
        }
    }

    // normally unreachable
    process::exit(1);
}

/// Set different logging levels for debug/release builds
fn log_level() -> log::Level {
    #[cfg(debug_assertions)]
    return log::Level::Debug;

    #[cfg(not(debug_assertions))]
    log::Level::Info
}
