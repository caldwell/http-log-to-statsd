extern crate logwatcher;
extern crate regex;
#[macro_use] extern crate lazy_static;

use logwatcher::LogWatcher;
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
struct Stats {
    status: Vec<usize>,
    status_major: Vec<usize>,
    verb: HashMap<String,usize>,
    https: usize,
    http: usize,
}

fn main() {
    let zeros = Stats {
        status: vec![0; 1000],
        status_major: vec![0; 6],
        verb: HashMap::new(),
        https: 0,
        http: 0,
    };
    let stats = Arc::new(Mutex::new(zeros.clone()));

    let verbose = false;
    let interval = 10;

    {
        let stats = stats.clone();
        thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::new(interval, 0));
                let mut s = stats.lock().unwrap();
                println!("   => {:?}", *s);
                *s = zeros.clone();
            }
        });
    }

    let mut log_watcher = LogWatcher::register("redacted-hardcoded-path-to/access.log".to_string()).unwrap();
    //let mut log_watcher = LogWatcher::register("redacted-hardcoded-path-to/access.log".to_string()).unwrap();
    log_watcher.watch(&|line| {
        lazy_static! {
            static ref GF_MATCHER: Regex = Regex::new(r#"(?P<client_ip>\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}) (?P<ident>[^ ]+) (?P<user>.+) \[(?P<time>\d{2}/.../\d{4}:\d{2}:\d{2}:\d{2} [-+]?\d{4})\]  (?P<scheme>https?) "(?:(?P<verb>\w+) (?P<request>[^ ]+)(?: HTTP/(?P<http_version>[0-9.]+))?|.*?)" (?P<resp_code>\d+) (?:(?P<resp_bytes>\d+)|-) .*"#).unwrap();
            static ref PORK_MATCHER: Regex = Regex::new(r#"(?P<client_ip>\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}) (?P<ident>[^ ]+) (?P<user>.+) \[(?P<time>\d{2}/.../\d{4}:\d{2}:\d{2}:\d{2} [-+]?\d{4})\] (?:(?P<scheme>\S+) )?"(?P<host>[^:]+):(?:(?P<verb>\w+) (?P<request>[^ ]+)(?: HTTP/(?P<http_version>[0-9.]+))?|.*?)" (?P<resp_code>\d+) (?:(?P<resp_bytes>\d+)|-) .*"#).unwrap();
        }
        if verbose { println!("{}", line) }
        if let Some(m) = GF_MATCHER.captures(&line) {
            if verbose { println!(" -> '{}' '{}' '{}' '{}' '{:?}' '{:?}' '{:?}' '{:?}' '{}' '{:?}'", &m["client_ip"], &m["ident"], &m["user"], &m["time"], &m.name("scheme"), &m.name("verb"), &m.name("request"), &m.name("http_version"), &m["resp_code"], &m.name("resp_bytes")) }
            let mut s = stats.lock().unwrap();
            let status = m["resp_code"].parse::<usize>().unwrap();
            s.status[status] += 1;
            s.status_major[status/100] += 1;
            if let Some(verb) = m.name("verb") {
                let old = *s.verb.get(verb).unwrap_or(&0);
                s.verb.insert(verb.to_string(), old + 1);
            }
            if m["scheme"] == "https".to_string() {
                s.https += 1;
            } else {
                s.http += 1;
            }
            // if  let Some(_) = m.name("scheme") {
            //     s.https += 1;
            // } else {
            //     s.http += 1;
            // }
        } else {
            println!(" !");
        }
//        println!("   => {:?}", *stats.lock().unwrap());
        // IP [0-9.]+
        // NUM [0-9]+
        // TEST %{IP:client_ip:tag} %{NUM:num:int}
        // SCHEME https?
        // GF_LOG_FORMAT %{CLIENT:client_ip} %{NGUSER:ident:drop} %{NGUSER:auth:drop} \[%{HTTPDATE:ts:ts-httpd}\]  %{SCHEME:scheme:tag} "(?:%{WORD:verb:tag} %{NOTSPACE:request:drop}(?: HTTP/%{NUMBER:http_version:float})?|%{DATA})" %{NUMBER:resp_code:int} (?:%{NUMBER:resp_bytes:int}|-) .*
    });
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
        try!(write!(f, "}}, http: {}, https: {}", self.http, self.https));
        Ok(())
    }
}
