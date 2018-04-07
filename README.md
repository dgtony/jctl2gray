# jctl2gray
Conversion of *journalctl* logs into Graylog Extended Log Format (GELF) and sending it to remote Graylog instance. Intended to be a replacement for the [journal2gelf](https://github.com/systemd/journal2gelf).


## Requirements
Works with the new versions of *systemd* (>= 190) supporting single-line JSON format output.


## Usage
There are two modes of operation:

* read JSON's from stdin: `journalctl -o json -f | jctl2gray -c config.toml`

* read directly from the subprocess with journalctl.

All variable parameters are set in the configuration file. Process will watch the file during operation, and changes of parameters in section `watched` will be applied immediately on the fly.


## Credits
Whole idea was taken from [journal2gelf](https://github.com/systemd/journal2gelf) project.

Module GELF was highly inspired by [gelf-rust](https://github.com/bzikarsky/gelf-rust) implementation, and partly reuse its code.
