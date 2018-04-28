# http-log-to-statsd

This program reads specialized custom logs from nginx/apache via UDP and
writes the data out to a statsd server. If the examples in this file are
used, The values passed to the statsd server are:

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

The log fields are now customizable using a (very terse) log notation (see
the 'Log Line Format' section).

*Note*: The examples shown here for configuring nginx and apache match the
stats from the older, hardcoded version of the program. If you previously
used that version then *you will need to update your nginx/apache
configurations to use the new log format*.

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

### Conditional Fields

Conditional Fields start with `?` and act like primitive 'if'
statements. There are 3 parts to the field, each separated by a `;`. The
first part is the predicate, the second is the 'if case' and the third is an
optional 'else case'.

The predicate is composed of a left value, an operator, and a right value
(in that order). The currently supported operators are `<`,`>`, and `=` for
less-than, greater-than, and equality, respectively. If either value
contains single-quotes then both values are parsed and compared as strings
(see Predicate String Quoting Rules, below). Otherwise, If either value has a `.` then
both values are parsed and compared as floats, otherwise they are parsed as
64 bit integers.

If the predicate is true then the 'if case' is evaluated. If it is false and
an 'else case' exists, the 'else case' is evaluated. The 'if' and 'else'
cases can be any field (other than another Conditional Field).

#### Predicate String Quoting Rules

Strings values in conditional field predicates must be surrounded by single
quotes and may not contain whitespace, `'`, `<`, `=`, or `>`
characters. This is mostly due to the really cheesy parser currently in use.

Examples:

`?-1<0;+key` will increment the `key` statsd bucket (since -1 is less than 0).

`?1<0;+key` will do nothing (since 1 is not less than 0 and there is no else
case)

`?500=500;+error` will increment the `error` statsd bucket (since 500 is
equal to 500).

`?'/api'='/api';+api` will increment the `api` statsd bucket (since the
strings are equal).

`?1.0<2;>_fast;>_slow +request` will increment the `request_fast` statsd
bucket (since 1 is less than 2 the `>_fast` field is evaluated, which sets
the suffix to `_fast`).

## Usage:

      http-log-to-statsd [-h | --help] [-v...] [--listen=<listen>] [--statsd=<server>] [--prefix=<prefix>]

### Options:

      -h --help                Show this screen.
      --listen=<listen>        Address and port number to listen on [default: 127.0.0.1:6666]
      --statsd=<server>        Address and port number of statsd server [default: 127.0.0.1:8125]
      --prefix=<prefix>        Statsd prefix for metrics [default: "http.request"]

# Author

Copyright © 2016-2018 David Caldwell <david@porkrind.org>

This program is free software: you can redistribute it and/or modify it
under the terms of the GNU General Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your option)
any later version.

This program is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
more details.

You should have received a copy of the GNU General Public License along with
this program.  If not, see <http://www.gnu.org/licenses/>.
