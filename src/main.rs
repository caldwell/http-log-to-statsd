extern crate rustc_serialize;
extern crate docopt;
extern crate cadence;

use docopt::Docopt;
use cadence::prelude::*;
use cadence::{StatsdClient, UdpMetricSink, DEFAULT_PORT};
use std::net::UdpSocket;

#[derive(Debug, RustcDecodable)]
struct Options {
    flag_v: isize,
    flag_listen: String,
    flag_statsd: String,
    flag_prefix: String,
    flag_suffix: String,
}

fn main() {
    let options = Options {
        flag_v: 0,
        flag_listen: "127.0.0.1:6666".to_string(),
        flag_statsd: format!("127.0.0.1:{}", DEFAULT_PORT),
        flag_prefix: "http.request".to_string(),
        flag_suffix: "".to_string(),
    };

        let usage = format!("
Usage:
  glf [-h | --help] [-v...] [--listen=<listen>] [--statsd=<server>] [--prefix=<prefix>] [--suffix=<suffix>]

Options:
  -h --help                Show this screen.
  --listen=<listen>        Address and port number to listen on [default: {}]
  --statsd=<server>        Address and port number of statsd server [default: {}]
  --prefix=<prefix>        Statsd prefix for metrics [default: {}]
  --suffix=<suffix>        Statsd suffix for metrics [default: {}]
", options.flag_listen, options.flag_statsd, options.flag_prefix, options.flag_suffix);

    let options: Options = Docopt::new(usage)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    if options.flag_v > 1 { println!("{:?}", options) }

    let verbose = options.flag_v;

    let statsd = StatsdClient::<UdpMetricSink>::from_udp_host(options.flag_prefix.as_str(), options.flag_statsd.as_str()).unwrap();
    let name = |name: &str| { format!("{}{}", name, options.flag_suffix) };

    // Read from webserver and accumulate stats
    let socket = UdpSocket::bind(options.flag_listen.as_str()).unwrap();
    let mut buf = [0; 512];
    loop {
        if let Ok((amt, _/*src*/)) = socket.recv_from(&mut buf) {
            if let Ok(line) = std::str::from_utf8(&buf[0..amt]).map(|s| s.to_string()) {
                if verbose > 1 { println!("{}", line) }
                // <190>Sep  3 15:40:50 deck nginx: http GET 200 751 498 0.042
                let line = if line.len() > 1 && line.chars().nth(0).unwrap() == '<' { // Strip off syslog gunk, if it exists
                    if let Some(start_byte) = line.find(": http").map(|l|l+2) {
                        std::str::from_utf8(&line.as_bytes()[start_byte..]).unwrap_or(line.as_str()).to_string()
                    } else { line }
                } else { line };
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() == 6 {
                    let (scheme, method, status, request_bytes, response_bytes, response_time_ms) = (fields[0], fields[1].to_lowercase(), fields[2], fields[3], fields[4], fields[5]);
                    if verbose > 1 { println!("{},{},{},{},{},{}", scheme, method, status, request_bytes, response_bytes, response_time_ms) }

                    if scheme == "https".to_string() { let _ = statsd.incr(&name("https")); }
                    else                             { let _ = statsd.incr(&name("http")); }

                    let _ = statsd.incr(&name(&method));

                    let _ = statsd.incr(&name(status));
                    let _ = statsd.incr(&name(&format!("{}xx", status.chars().nth(0).unwrap_or('X'))));

                    let _ = statsd.time(&name("request_bytes"),    request_bytes   .parse::<u64>().unwrap_or(0)); // looks wrong, but times get averaged, which is correct for bytes.
                    let _ = statsd.time(&name("response_bytes"),   response_bytes  .parse::<u64>().unwrap_or(0));
                    let _ = statsd.time(&name("response_time_ms"), if response_time_ms.contains('.') { (response_time_ms.parse::<f64>().unwrap_or(0.0) * 1000.0) as u64 } // ngingx
                                                                   else                              { response_time_ms.parse::<u64>().unwrap_or(0) });               // apache
                    let _ = statsd.incr(&name("requests"));
                } else if verbose > 0 { println!("!{}", line) }
            }
        }
    }
}

