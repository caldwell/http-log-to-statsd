extern crate rustc_serialize;
extern crate docopt;

use docopt::Docopt;
use std::net::UdpSocket;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, RustcDecodable)]
struct Options {
    flag_v: isize,
    flag_interval: u64,
    flag_listen: String,
}

#[derive(Clone)]
struct Stats {
    status: Vec<u64>,
    status_major: Vec<u64>,
    verb: HashMap<String,u64>,
    https: u64,
    http: u64,
    request_bytes: u64,
    response_bytes: u64,
    response_time_ms: u64,
    requests: u64,
}

fn main() {
    let options = Options {
        flag_v: 0,
        flag_interval: 60,
        flag_listen: "127.0.0.1:6666".to_string(),
    };

        let usage = format!("
Usage:
  glf [-h | --help] [-v...] [--interval=<interval>] [--listen=<listen>]

Options:
  -h --help                Show this screen.
  --interval=<interval>    Accumulation period in seconds [default: {}]
  --listen=<listen>        Address and port number to listen on [default: {}]
", options.flag_interval, options.flag_listen);

    let options: Options = Docopt::new(usage)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    if options.flag_v > 1 { println!("{:?}", options) }


    let zeros = Stats {
        status: vec![0; 1000],
        status_major: vec![0; 6],
        verb: HashMap::new(),
        https: 0,
        http: 0,
        request_bytes: 0,
        response_bytes: 0,
        response_time_ms: 0,
        requests: 0,
    };
    let stats = Arc::new(Mutex::new(zeros.clone()));

    let verbose = false;

    // Dump stats to ??? every once and a while
    {
        let stats = stats.clone();
        let interval = options.flag_interval;
        thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::new(interval, 0));
                let mut s = stats.lock().unwrap();
                println!("   => {:?}", *s);
                *s = zeros.clone();
            }
        });
    }

    // Read from webserver and accumulate stats
    let mut socket = UdpSocket::bind(options.flag_listen.as_str()).unwrap();
    let mut buf = [0; 512];
    loop {
        if let Ok((amt, src)) = socket.recv_from(&mut buf) {
            if let Ok(line) = std::str::from_utf8(&buf[0..amt]).map(|s| s.to_string()) {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() == 6 {
                    let (scheme, method, status, request_bytes, response_bytes, response_time_ms) = (fields[0], fields[1], fields[2], fields[3], fields[4], fields[5]);
                    println!("{},{},{},{},{},{}", scheme, method, status, request_bytes, response_bytes, response_time_ms);

                    let mut s = stats.lock().unwrap();

                    if scheme == "https".to_string() { s.https += 1;}
                    else                             { s.http  += 1; }

                    let old = *s.verb.get(method).unwrap_or(&0);
                        s.verb.insert(method.to_string(), old + 1);

                    let status = status.parse::<usize>().unwrap_or(0);
                    s.status[status] += 1;
                    s.status_major[status/100] += 1;

                    s.request_bytes += request_bytes.parse::<u64>().unwrap_or(0);
                    s.response_bytes += response_bytes.parse::<u64>().unwrap_or(0);
                    s.response_time_ms += response_time_ms.parse::<u64>().unwrap_or(0);
                    s.requests += 1;
                } else { println!("{}", line) }
            }
        }
    }
    println!("Goodbye, cruel world!");
}

impl std::fmt::Debug for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        try!(write!(f, "status{{"));
        let mut something = false;
        for (i,s) in self.status.iter().enumerate() {
            if *s > 0 {
                if something { try!(write!(f, " ")) }
                try!(write!(f, "{}: {}", i, s));
                something = true;
            }
        }
        try!(write!(f, "}}, status_major{{"));
        let mut something = false;
        for (i,s) in self.status_major.iter().enumerate() {
            if *s > 0 {
                if something { try!(write!(f, " ")) }
                try!(write!(f, "{}xx: {}", i, s));
                something = true;
            }
        }
        try!(write!(f, "}}, verb{{"));
        let mut something = false;
        for (i,s) in self.verb.iter() {
            if *s > 0 {
                if something { try!(write!(f, " ")) }
                try!(write!(f, "{}: {}", i, s));
                something = true;
            }
        }
        try!(write!(f, "}}, http: {}, https: {}, request_bytes: {}, response_bytes: {}, response_time_ms: {}, requests: {}",
                    self.http, self.https, self.request_bytes, self.response_bytes, self.response_time_ms, self.requests ));
        Ok(())
    }
}
