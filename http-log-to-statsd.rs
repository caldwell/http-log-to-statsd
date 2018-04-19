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
}

fn main() {
        let usage = format!("
Usage:
  http-log-to-statsd [-h | --help] [-v...] [--listen=<listen>] [--statsd=<server>] [--prefix=<prefix>]

Options:
  -h --help                Show this screen.
  -v                       Increase verbosity.
  --listen=<listen>        Address and port number to listen on [default: 127.0.0.1:6666]
  --statsd=<server>        Address and port number of statsd server [default: 127.0.0.1:{}]
  --prefix=<prefix>        Statsd prefix for metrics [default: http.request]
", DEFAULT_PORT);

    let options: Options = Docopt::new(usage)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    if options.flag_v > 1 { println!("{:?}", options) }

    let verbose = options.flag_v;

    let statsd = StatsdClient::<UdpMetricSink>::from_udp_host(options.flag_prefix.as_str(), options.flag_statsd.as_str()).unwrap();

    // Read from webserver and accumulate stats
    let socket = UdpSocket::bind(options.flag_listen.as_str()).unwrap();
    let mut buf = [0; 512];
    let mut parser = Parser::new();
    loop {
        if let Ok((amt, _/*src*/)) = socket.recv_from(&mut buf) {
            if let Ok(line) = std::str::from_utf8(&buf[0..amt]).map(|s| s.to_string()) {
                if verbose > 1 { println!("{}", line) }

                let stats = parser.parse_line(line);
                for stat in stats {
                    match stat {
                        Stat::Incr(name)    => { let _ = statsd.incr(&name); },
                        Stat::Avg(name,val) => { let _ = statsd.time(&name, val); },
                    }
                }
            }
        }
    }
}


#[derive(Debug, PartialEq)]
pub enum Stat {
    Incr(String),
    Avg(String,u64),
}

#[derive(Debug)]
pub struct Parser {
    suffix: String,
}

impl Parser {
    pub fn new() -> Parser {
        Parser{suffix: "".to_string()}
    }
    pub fn parse_line(&mut self, line: String) -> Vec<Stat> {
        let mut stats = Vec::new();
        // <190>Sep  3 15:40:50 deck nginx: http GET 200 751 498 0.042 extra.suffix
        let line = if line.len() > 1 && line.chars().nth(0).unwrap() == '<' { // Strip off syslog gunk, if it exists
            if let Some(start_byte) = line.find(": http").map(|l|l+2) {
                std::str::from_utf8(&line.as_bytes()[start_byte..]).unwrap_or(line.as_str()).to_string()
            } else { line }
        } else { line };
        self.suffix = "".to_string();
        let name = |name: &str, suffix: &str| { [name, suffix].concat() };
        for field in line.split_whitespace() {
            if field.len() < 2 { continue }
            match field.chars().nth(0).unwrap_or(' ') {
                '+' => { /* +GET +200 */ let _ = stats.push(Stat::Incr(name(&field[1..], &self.suffix))); },
                'x' => { /* +GET x200 */ let _ = stats.push(Stat::Incr(name(&format!("{}xx", field.chars().nth(1).unwrap_or('X')), &self.suffix))); },
                '~' => { /* ~request_bytes:501   ~response_time_ms:1.52*1000 */
                             let x: Vec<&str> = field[1..].splitn(2, ':').collect();
                             if x.len() == 2 {
                                 let (key, mut value, mut scale) = (x[0], x[1], "1");
                                 if value.contains("*") {
                                     let x: Vec<&str> = field[1..].splitn(2, '*').collect();
                                     value = x[1];
                                     scale = x[0];
                                 }
                                 let _ = stats.push(Stat::Avg(name(key, &self.suffix),
                                                              if value.contains('.') || scale.contains('.') { (value.parse::<f64>().unwrap_or(0.0) * scale.parse::<f64>().unwrap_or(1.0)) as u64 }
                                                              else                                          { value.parse::<u64>().unwrap_or(0)    * scale.parse::<u64>().unwrap_or(1)  }));
                             } else {
                                 println!("Couldn't parse average(~) field: {}", field)
                             }
                },
                '>' => { self.suffix = field[1..].to_string() },
                _ => { println!("Unknown field: {}", field) }
            }
        }
        stats
    }
}
