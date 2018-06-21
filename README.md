# jctl2gray
Conversion of *journalctl* logs into Graylog Extended Log Format (GELF) and sending it to remote Graylog instance. Intended to be a replacement for the [journal2gelf](https://github.com/systemd/journal2gelf).


## Requirements
Works with the new versions of *systemd* (>= 190) supporting single-line JSON format output.


## Usage
There are two modes of operation:

* read JSON's from stdin: `journalctl -o json -f | jctl2gray -s stdin`

* read directly from the subprocess with journalctl: `jctl2gray -s journal -t graylog.domain.com:9000`

All parameters are defined via command line arguments, run `jctl2gray --help` for detailed information.

### Additional fields
Sometimes you need to attach some informational fields, e.g. in order to organize distinct streams in Graylog. This could be easily done by using `--opt` with comma-separated arguments in the following format: `field_name=field_text`.

For example option `--opt team=core,service=backend` will produce messages with two additional fields: `"team":"core"` and `"service":"backend"`.


### Filter logs
Journal could be filtered by logging levels on two tiers: systemd's priority and message logging level.

One can easily set logging level threshold for systemd: e.g. if option `--sys warning` was provided, then messages with priority `notice`, `info` and `debug` won't be sent to Graylog.

Option `--msg` could be used to filter stream with internal message logging level, trying to find pattern `level=some_level` in the body of the message. If pattern found, then it will be compared with predefined threshold and either being sent to Graylog or dropped. Feature plays well with structured loggers, (e.g. Go's [logrus](https://github.com/sirupsen/logrus)).


## Credits
Whole idea was taken from [journal2gelf](https://github.com/systemd/journal2gelf) project.

Module GELF was highly inspired by [gelf-rust](https://github.com/bzikarsky/gelf-rust) implementation, and partly reuse its code.
