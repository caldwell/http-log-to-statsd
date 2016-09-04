# http-log-to-statsd

## Configuring nginx:

    log_format stats '$scheme $request_method $status $request_length $body_bytes_sent $request_time';
    access_log syslog:server=127.0.0.1:6666 stats;

## Configuring apache 2.4 and later (requires netcat to be installed):

    # https method status request-size(bytes) response-size(bytes) request-time(ms)
    LogFormat "%{REQUEST_SCHEME}x %m %>s %I %O %{ms}T" stats
    CustomLog "|/bin/nc -u localhost 6666" stats

## Usage:

      http-log-to-statsd [-h | --help] [-v...] [--listen=<listen>] [--statsd=<server>] [--prefix=<prefix>] [--suffix=<suffix>]

### Options:

      -h --help                Show this screen.
      --listen=<listen>        Address and port number to listen on [default: 127.0.0.1:6666]
      --statsd=<server>        Address and port number of statsd server [default: 127.0.0.1:8125]
      --prefix=<prefix>        Statsd prefix for metrics [default: "http.request"]
      --suffix=<suffix>        Statsd suffix for metrics [default: ""]
