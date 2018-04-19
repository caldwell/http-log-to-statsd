# http-log-to-statsd

This program reads specialized custom logs from nginx/apache via UDP and
writes the data out to a statsd server. The values passed to the statsd
server are:

- `http.request.{https,http}`: counts of requests coming in on https or http
- `http.request.{get,post,put,head,options,etc}`: counts of http method
- `http.request.{100-500}`: counts of each http status code
- `http.request.{1xx,2xx,3xx,4xx,5xx}`: counts of each http status category
- `http.request.request_bytes`: bytes in each request
- `http.request.response_bytes`: bytes in each response
- `http.request.request_time_ms`: time in ms for each request to complete
- `http.request.requests`: count of requests

The `http.request` prefix can by changed with the `--prefix` command line
option. In addition, the incoming log line can specify a suffix in the 7th
field, which may be used to add arbitrary suffixes to the stat names. For
instance, telgraf/influx users might want to add `",sometag=somevalue"` to
inject custom tags.

## Building from source:

Building the code requires [Rust](https://www.rust-lang.org). To build:

    $ cargo build --release

The resulting executable will be in `target/release/http-log-to-statsd`.

## Configuring nginx:

    log_format stats '+$scheme +$request_method +$status x$status ~request_bytes:$request_length ~response_bytes:$body_bytes_sent ~response_time_ms:$request_time*1000 +requests';
    access_log syslog:server=127.0.0.1:6666 stats;

To add an arbitrary suffix to each metric name, add one more field to the
start of log_format:

    log_format stats '>.$host +$scheme +$request_method +$status x$status ~request_bytes:$request_length ~response_bytes:$body_bytes_sent ~response_time_ms:$request_time*1000 +requests';

If you use Telegraf/InfluxDB, you might want to use InfluxDB's tag format:

    log_format stats '>,host=$host +$scheme +$request_method +$status x$status ~request_bytes:$request_length ~response_bytes:$body_bytes_sent ~response_time_ms:$request_time*1000 +requests';

## Configuring apache 2.4 and later (requires netcat to be installed):

    # https method status request-size(bytes) response-size(bytes) request-time(ms)
    LogFormat "+%{REQUEST_SCHEME}x +%m +%>s x%>s ~request_bytes:%I ~response_bytes:%O ~response_time_ms:%{ms}T +requests"
    CustomLog "|/bin/nc -u localhost 6666" stats

To add an arbitrary suffix to each metric name, add one more field to the
start of LogFormat:

    # hostname/extra-suffix https method status request-size(bytes) response-size(bytes) request-time(ms)
    LogFormat ">.%v +%{REQUEST_SCHEME}x +%m +%>s x%>s ~request_bytes:%I ~response_bytes:%O ~response_time_ms:%{ms}T +requests"

If you use Telegraf/InfluxDB, you might want to use InfluxDB's tag format:

    # https method status request-size(bytes) response-size(bytes) request-time(ms) hostname/extra-suffix
    LogFormat ">,host=%v +%{REQUEST_SCHEME}x +%m +%>s x%>s ~request_bytes:%I ~response_bytes:%O ~response_time_ms:%{ms}T +requests"

## Log Line Format

The log line is fully configurable. Each log line consists of several fields
separated by whitespace. The fields are described below. Examples are from
the perspective of the this program--in other words, the log lines as they
are written to the socket, not as they are specified in the server
configuration.

Example:

    >.example.com +https +GET +200 x200 ~request_bytes:120 ~response_bytes:800 ~response_time_ms:0.50*1000 +requests

### Counting Fields

Counting fields start with a `+`. The remainder of the field is used as the
statsd "bucket" for count.

Example: `+GET` will increment the "GET" statsd bucket.

### Status Bucket Fields

Status Bucket Fields start with `x`. Only the first character following the
`x` is used. It is suffixed with `xx` and used as a counting metric in
statsd. This is meant for http status codes, so they can be counted in wide
groups (4xx, 5xx, etc).

Example: `x502` will increment the "5xx" statsd bucket.

### Averaging Fields

Averaging Fields come in one of 2 forms:

    ~key:value
    ~key:value*scale

`key` may not contain `:`. `value` may either be a float or an integer (but
floats are truncated to integers after scaling). `value` is multiplied by
`scale` before being given to statsd and is assumed to be 1 when not
given. `scale` is most useful for nginx request_time metrics which are given
in fractional seconds.

Statsd calls these "Timing metrics" but since they are just averaged it
doesn't really help to think of them as times. Byte counts, for instance,
are perfectly valid.

Examples:

`~response_time_ms:0.050*1000` will give a value of 50 to the
"response_time_ms" statsd metric.

`~response_bytes:800` will give a value of 800 to the "response_bytes"
statsd metric.

### Suffix Fields

Suffix Fields start with '>'. The remainder of the field sets a suffix to be
used in the remaining metrics. The fields are processed in order from left
to right, so if you want the suffix to apply to all the metrics, you must
put the suffix field first. You may have multiple suffix fields in a log
line--they will apply to the fields to their right until the next suffix
field (or the end of the line).

Example: `>.example.com` will append `.example.com` to all metrics
processed after this field.

## Usage:

      http-log-to-statsd [-h | --help] [-v...] [--listen=<listen>] [--statsd=<server>] [--prefix=<prefix>]

### Options:

      -h --help                Show this screen.
      --listen=<listen>        Address and port number to listen on [default: 127.0.0.1:6666]
      --statsd=<server>        Address and port number of statsd server [default: 127.0.0.1:8125]
      --prefix=<prefix>        Statsd prefix for metrics [default: "http.request"]
