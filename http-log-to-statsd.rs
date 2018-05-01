// Copyright © 2016-2018 David Caldwell <david@porkrind.org>
// Licensed under the GPL v3 or newer. See the LICENSE file for details.

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

                let stats = parser.parse_line(&line);
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
    pub fn parse_line(&mut self, line: &str) -> Vec<Stat> {
        let mut stats = Vec::new();
        // <190>Sep  3 15:40:50 deck nginx: http GET 200 751 498 0.042 extra.suffix
        let line = if line.len() > 1 && line.chars().nth(0).unwrap() == '<' { // Strip off syslog gunk, if it exists
            if let Some(start_byte) = line.find(": http").map(|l|l+2) {
                std::str::from_utf8(&line.as_bytes()[start_byte..]).unwrap_or(line).to_string()
            } else { line.to_string() }
        } else { line.to_string() };
        self.suffix = "".to_string();
        for field in line.split_whitespace() {
            match self.parse_field(field) {
                Ok(Some(stat)) => { stats.push(stat); },
                Ok(None) => {},
                Err(e) => { println!("{} in {}", e, field) },
            }
        }
        stats
    }
    fn parse_field(&mut self, field: &str) -> Result<Option<Stat>,String> {
        let name = |name: &str, suffix: &str| { [name, suffix].concat() };
        if field.len() == 0 { return Ok(None) }
        if field.len() < 2 { return Err(format!("field is too short ({})", field.len())) }
        match field.chars().nth(0).unwrap_or(' ') {
            '+' => { /* +GET +200 */ Ok(Some(Stat::Incr(name(&field[1..], &self.suffix)))) },
            'x' => { /* +GET x200 */ Ok(Some(Stat::Incr(name(&format!("{}xx", field.chars().nth(1).unwrap_or('X')), &self.suffix)))) },
            '~' => { /* ~request_bytes:501   ~response_time_ms:1.52*1000 */
                         match parse_optional_scaled_num(&field[1..])? {
                             (key, Some(value)) => Ok(Some(Stat::Avg(name(key, &self.suffix), value as u64))),
                             (_,   None)        => Err(format!("Average field is missing value: {}", field)),
                         }
            },
            '>' => { self.suffix = field[1..].to_string(); Ok(None) },
            '?' => {
                let x: Vec<&str> = field[1..].splitn(3, ';').collect();
                if x.len() != 2 && x.len() != 3 { return Err(format!("'?' should have 2 or 3 args and not {}", x.len())) }
                let (pred, ifcase, mut elsecase) = (x[0], x[1], if x.len() == 3 {x[2]} else {""});
                if let Some(op_index) = pred.find(|c| c=='<' || c=='>' || c=='=') {
                    let (l,op_r) = pred.split_at(op_index);
                    let (op, r) = (op_r.chars().nth(0).unwrap(), op_r.get(1..).unwrap());
                    let val = if l.contains('\'') || r.contains('\'') {
                        compare(parse_string(l), parse_string(r), op, l,r,"string")
                    } else if l.contains('.') || r.contains('.') {
                        compare(l.parse::<f64>(), r.parse::<f64>(), op, l,r,"float")
                    } else {
                        compare(l.parse::<i64>(), r.parse::<i64>(), op, l,r,"integer")
                    };
                    if val? { self.parse_field(ifcase).map_err(|e| format!("{} in if case", e)) }
                    else    { self.parse_field(elsecase).map_err(|e| format!("{} in else case", e)) }
                } else {
                    Err(format!("Couldn't find operator in predicate '{}'", pred))
                }
            },
            _ => { Err(format!("Unknown field: {}", field)) }
        }
    }
}

fn compare<T: PartialOrd, E>(l: Result<T,E>, r: Result<T,E>, op: char, ls: &str, rs: &str, kind: &str) -> Result<bool,String> {
    match (l, r, op) {
        (Ok(l), Ok(r), '<') => Ok(l < r),
        (Ok(l), Ok(r), '>') => Ok(l > r),
        (Ok(l), Ok(r), '=') => Ok(l == r),
        (Err(_),_,     _) => { Err(format!("Couldn't parse '{}' as {}", ls, kind)) }
        (_,     Err(_),_) => { Err(format!("Couldn't parse '{}' as {}", rs, kind)) }
        (_,_,_) => panic!("Can't happen: {}", op)
    }
}

fn parse_string(s: &str) -> Result<&str, String> {
    if s.starts_with("'") && s.ends_with("'") {
        Ok(&s[1..s.len()-1])
    } else {
        Err(format!("Bad string: {}", s))
    }
}

fn parse_optional_scaled_num(s: &str) -> Result<(&str,Option<i64>),String> { // xxx:1.5*2 -> Ok("xxx",Some(3))
    let x: Vec<&str> = s.splitn(2, ':').collect();
    if x.len() == 1 {
        Ok((x[0], None))
    } else if x.len() == 2 {
        let (key, mut value, mut scale) = (x[0], x[1], "1");
        if value.contains("*") {
            let x: Vec<&str> = value.splitn(2, '*').collect();
            value = x[1];
            scale = x[0];
        }
        Ok((key, Some(if value.contains('.') || scale.contains('.') { (value.parse::<f64>().unwrap_or(0.0) * scale.parse::<f64>().unwrap_or(1.0)) as i64 }
                      else                                          { value.parse::<i64>().unwrap_or(0)    * scale.parse::<i64>().unwrap_or(1)  })))
    } else {
        Err(format!("Couldn't parse field: {}", s))
    }
}

#[cfg(test)]
mod tests {
    fn parse_line(line: &str) -> Vec<::Stat> {
        let mut p = ::Parser::new();
        p.parse_line(line)
    }
    fn stat_incr(key: &str)            -> ::Stat { ::Stat::Incr(key.to_string()) }
    fn stat_count(key: &str, val: i64) -> ::Stat { ::Stat::Count(key.to_string(), val) }
    fn stat_avg(key: &str, val: u64)   -> ::Stat { ::Stat::Avg(key.to_string(), val) }

    #[test]
    fn incr() {
        let stats = parse_line("+david");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0],stat_incr("david"));
    }

    #[test]
    fn incr_xx() {
        let stats = parse_line("+501 x501 x502");
        assert_eq!(stats.len(), 3);
        assert_eq!(stats[0],stat_incr("501"));
        assert_eq!(stats[1],stat_incr("5xx"));
        assert_eq!(stats[2],stat_incr("5xx"));
    }

    #[test]
    fn avg() {
        let stats = parse_line("~david:42 ~david:42.0 ~david:7*6 ~david:7.0*6 ~david:7*6.0");
        assert_eq!(stats.len(), 5);
        assert_eq!(stats[0], stat_avg("david", 42));
        assert_eq!(stats[1], stat_avg("david", 42));
        assert_eq!(stats[2], stat_avg("david", 42));
        assert_eq!(stats[3], stat_avg("david", 42));
        assert_eq!(stats[4], stat_avg("david", 42));
    }

    #[test]
    fn suffix() {
        let stats = parse_line("+david >_rules +david >_is_so_great ~david:123 +david_definitely");
        assert_eq!(stats.len(), 4);
        assert_eq!(stats[0],stat_incr("david"));
        assert_eq!(stats[1],stat_incr("david_rules"));
        assert_eq!(stats[2], stat_avg("david_is_so_great", 123));
        assert_eq!(stats[3],stat_incr("david_definitely_is_so_great"));
    }

    #[test]
    fn parse_state() {
        let mut p = ::Parser::new();
        let stats = p.parse_line(">_is_great +david");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0],stat_incr("david_is_great"));
        let stats = p.parse_line("+david");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0],stat_incr("david"));
    }

    #[test]
    fn if_then_else() {
        let stats = parse_line("?0<1;+a ?0>1;+b ?7.2<7.3;+c;+d ?6.9>7;+e;+f ?-1<0;>_x;>_y ?-3.14>-4;~sandwich:10*5.0;~lemonade:13*69.2 ?< ?3< ?<3 ?1<0 ?1<0;; ?1<0;;x ?x<y;nope");
        assert_eq!(stats.len(), 4);
        assert_eq!(stats[0],stat_incr("a"));
        assert_eq!(stats[1],stat_incr("c"));
        assert_eq!(stats[2],stat_incr("f"));
        assert_eq!(stats[3], stat_avg("sandwich_x", 50));
    }

    #[test]
    fn if_numeric_equal() {
        let stats = parse_line("?0=1;+a ?1=1;+b ?1.0=1;+c ?1.1=1.100;+d ?2.5=2.0;+e");
        assert_eq!(stats.len(), 3);
        assert_eq!(stats[0],stat_incr("b"));
        assert_eq!(stats[1],stat_incr("c"));
        assert_eq!(stats[2],stat_incr("d"));
    }

    #[test]
    fn if_strings() {
        let stats = parse_line("?'0'='1';+a ?'1'='1';+b ?'1.0'='1';+c ?'1.1'='1.100';+d ?'this'='that';+e ?'this'='this';+f");
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0],stat_incr("b"));
        assert_eq!(stats[1],stat_incr("f"));
        let stats = parse_line("?bad'=bad';+g ?'bad='bad;+h ?'good='='good=';+i ?'str'>0;+j");
        assert_eq!(stats.len(), 0);
        //assert_eq!(stats[0],stat_incr("i")); // FIXME: write a better parser!!
        let stats = parse_line("?'a'<'A';+j ?'David'<'david';+david ?'c'>'a';+rules ?'d'>'e';+nope");
        assert_eq!(stats.len(), 2);
        assert_eq!(stats[0],stat_incr("david"));
        assert_eq!(stats[1],stat_incr("rules"));
    }
}
