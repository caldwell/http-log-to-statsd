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
option. In addition, the `--suffix` options may be used to add arbitrary
suffixes to the stat names. For instance, telgraf/influx users might want to
add `",sometag=somevalue"` to inject custom tags.

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
