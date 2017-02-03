#![allow(unused_mut, unused_variables, unused_must_use)]

extern crate regex;
extern crate rand;
extern crate clap;
extern crate dns;

use regex::Regex;
use clap::{Arg, App};
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::fs::File;
use std::net::{UdpSocket, Ipv4Addr, SocketAddrV4, TcpListener, TcpStream, IpAddr};
use std::thread;
use std::time::Duration;
#[cfg(not(windows))]
use std::os::unix::prelude::*;
#[cfg(windows)]
use std::os::windows::prelude::*;

fn main() {

    let matches = App::new("eHosts")
        .about("An ehanced hosts file")
        .arg(Arg::with_name("file")
             .short("f")
             .help("Specify rule file, [default: ./hosts]")
             .takes_value(true))
        .arg(Arg::with_name("addr")
             .short("d")
             .multiple(true)
             .help("Set upstream DNS server [default: 8.8.8.8:53]")
             .takes_value(true))
        .arg(Arg::with_name("mode")
             .short("s")
             .help("run in server mode"))
        .arg(Arg::with_name("tcp")
             .short("t")
             .help("query using tcp for udp request"))
        .get_matches();

    let local =  if matches.is_present("mode") {
        println!("Run eHost in servers mode"); 
        "0.0.0.0:53"
    }else{
        println!("Run eHost in local mode"); 
        "127.0.0.1:53"
    };

    let up_dns = matches.values_of("addr").map_or(vec!["8.8.8.8:53".to_string()], |s| {
        s.map(|a| {
            if a.contains(":") {
                a.to_string()
            }else{
                format!("{}:53", a)
            }
        }).collect()
    });

    println!("Upstream DNS is {:?}", up_dns);

    let is_tcp = matches.is_present("tcp");

    let path = matches.value_of("file").unwrap_or("./hosts").to_string();
    println!("The hosts file is '{}'", path);

    let mut mtime = -1;

    if cfg!(windows) {
        println!("Warn: auto set dns is not support in Windows, please set dns manually!");
    }else{
        set_dns();
    }

    //listen tcp
    {
        let up_dns = up_dns.clone();
        let path = path.clone();
        thread::spawn(move || {
            let mut rules: Vec<Rule> = vec![];
            check_rule_update(&path, &mut mtime, &mut rules);
            let mut local_socket = TcpListener::bind(local).unwrap();
            loop {
                match local_socket.accept(){
                    Ok((mut stream, _)) => {
                        check_rule_update(&path, &mut mtime, &mut rules);

                        let local_socket = local_socket.try_clone().unwrap();
                        let rules = rules.clone();
                        let up_dns = up_dns.clone();
                        thread::spawn(move || {
                            let mut buf = [0u8; 512];
                            let mut len = stream.read(&mut buf).unwrap();
                            let mut dns_msg = dns::to_dns(&buf[..len], "tcp");

                            //thread::sleep_ms(10000);

                            dns_msg.head.qe = 256;
                            dns_msg.head.anc = 0;
                            dns_msg.head.nsc = 0;
                            dns_msg.head.arc = 0;

                            if match_rule(&mut dns_msg, &rules) {
                                let (buf, len) = dns::from_dns(&dns_msg, "tcp");
                                stream.write_all(&buf[..len]);
                                return;
                            }


                            for dns in up_dns {
                                let mut dns_socket = TcpStream::connect(&*dns).unwrap();
                                dns_socket.write_all(&buf[..len]);
                                dns_socket.set_read_timeout(Some(Duration::from_millis(500)));

                                match dns_socket.read(&mut buf){
                                    Ok(len) => {
                                        stream.write_all(&buf[..len]);

                                        println!("{} doesn't match any rules, response from {}", &dns_msg.ques[0].qname.join("."), dns);
                                        break;
                                    },
                                    Err(e) => {
                                        println!("{} dns {} timeout {}", &dns_msg.ques[0].qname.join("."), dns, e);
                                    },
                                };
                            }
                        });
                    }
                    Err(e) => {
                        println!("An err: {}",e);
                        //unreachable!()
                    }
                };
            };
        });
    }

    //listen udp
    thread::spawn(move || {
        let mut rules: Vec<Rule> = vec![];
        let mut local_socket = UdpSocket::bind(local).unwrap();
        let mut buf = [0u8; 512];
        loop {
            //let recv = local_socket.recv_from(&mut buf); 
            match local_socket.recv_from(&mut buf){
                Ok((len, src)) => {
                    check_rule_update(&path, &mut mtime, &mut rules);

                    let local_socket = local_socket.try_clone().unwrap();
                    let rules = rules.clone();
                    let up_dns = up_dns.clone();
                    thread::spawn(move || {
                        //dns::show_dns(&buf[..len]);
                        let mut dns_msg = dns::to_dns(&buf[..len], "udp");
                        //println!("{:?}", dns_msg);
                        //io::stdout().flush();

                        //thread::sleep_ms(10000);

                        dns_msg.head.qe = 256;
                        dns_msg.head.anc = 0;
                        dns_msg.head.nsc = 0;
                        dns_msg.head.arc = 0;

                        if match_rule(&mut dns_msg, &rules) {
                            let (buf, len) = dns::from_dns(&dns_msg, "udp");
                            local_socket.send_to(&buf[..len], src);
                            return;
                        }

                        if is_tcp {
                            let (mut buf, len) = dns::from_dns(&dns_msg, "tcp");
                            for dns in up_dns {
                                let mut dns_socket = TcpStream::connect(&*dns).unwrap();
                                dns_socket.write_all(&buf[..len]);
                                dns_socket.set_read_timeout(Some(Duration::from_millis(500)));

                                match dns_socket.read(&mut buf){
                                    Ok(len) => {
                                        local_socket.send_to(&buf[2..len], src);

                                        println!("{} doesn't match any rules, response from {}", &dns_msg.ques[0].qname.join("."), dns);
                                        break;
                                    },
                                    Err(e) => {
                                        println!("{} dns {} timeout {}", &dns_msg.ques[0].qname.join("."), dns, e);
                                    },
                                };
                            }
                        }else{
                            let mut dns_socket = random_udp(Ipv4Addr::new(0, 0, 0, 0));

                            //set timeout
                            //dns_socket.set_read_timeout(Some(Duration::from_millis(500)));
                            dns_socket.set_read_timeout(Some(Duration::from_millis(500)));

                            for dns in up_dns {
                                dns_socket.send_to(&buf[..len], &*dns);

                                match dns_socket.recv_from(&mut buf){
                                    Ok((len, _)) => {
                                        local_socket.send_to(&buf[..len], src);

                                        //dns::show_dns(&buf[..len]);
                                        //println!("{:?}", &buf[..len]);
                                        //println!("{:?}", dns::to_dns(&buf));
                                        //drop(dns_socket);
                                        println!("{} doesn't match any rules, response from {}", &dns_msg.ques[0].qname.join("."), dns);
                                        break;
                                    },
                                    Err(e) => {
                                        println!("{} dns {} timeout {}", &dns_msg.ques[0].qname.join("."), dns, e);
                                    },
                                };
                            }
                        }
                    });
                }
                Err(e) => {
                    println!("An err: {}",e);
                    //unreachable!()
                }
            };
        };
    }).join();
}

#[cfg(windows)]
trait MyMtime {
    fn mtime(&self) -> isize;
}

#[cfg(windows)]
impl MyMtime for std::fs::Metadata {
    fn mtime(&self) -> isize {
        self.last_write_time() as isize
    }
}

#[derive(Debug, Clone)]
struct Rule {
    ip: IpAddr,
    patt: Regex,
}

fn set_dns() {
    let file = File::open("/etc/resolv.conf").unwrap();
    let mut lines : Vec<_> = BufReader::new(&file).lines().map(|x| x.unwrap()).collect();
    let mut writer = BufWriter::new(File::create("/etc/resolv.conf").unwrap());
    let mut i=0;
    for line in lines {
        if line.starts_with("nameserver") {
            i+=1;
            if i==1 {
                if let None = line.find("127.0.0.1") {
                    writer.write_fmt(format_args!("{}\n", "nameserver 127.0.0.1"));
                }
            }
        }
        writer.write_fmt(format_args!("{}\n", line));
    }
    if i==0 {
        writer.write_fmt(format_args!("{}\n", "nameserver 127.0.0.1"));
        //i+=1;
    }
    //if i==1 {
    //writer.write_fmt(format_args!("{}\n", "nameserver 8.8.8.8"));
    //}
    println!("changed dns setting to 127.0.0.1")
}

fn parse_rule(file: &File) -> Vec<Rule> {
    let mut rules: Vec<Rule> = Vec::new();
    let gm = Regex::new(r"\s+").unwrap();

    let mut buf = String::new();
    BufReader::new(file).read_to_string(&mut buf);

    for l in buf.lines() {
        if l.starts_with("#$") {
            let mut split = gm.splitn(l.trim_left_matches("#$").trim(), 100);
            let ip = (split.nth(0).unwrap()).parse().unwrap();
            for i in split {
                rules.push(Rule{ip: ip, patt: Regex::new(i).unwrap()});
            }
        }
    }

    println!(", The Rules is");
    for r in &rules {
        println!("{:?}", r);
    }

    rules
}

fn check_rule_update(path: &str, mtime: &mut isize, rules: &mut Vec<Rule>) {
    let mut file = match File::open(&path) {
        Ok(file) => {
            file
        }
        Err(_) => {
            panic!("File '{}' doesn't exit! Please create it or specify a rules file via '-f' option", path);
        }
    };
    if *mtime != file.metadata().unwrap().mtime() as isize {
        *mtime = file.metadata().unwrap().mtime() as isize;
        *rules = parse_rule(&file);
        if rules.len() == 0 {
            println!("Warn: file '{}' doesn't contain any rules!", path);
        }
        println!("rules updated");
    }
}

fn match_rule(dns_msg: &mut dns::DnsMsg, rules: &Vec<Rule>) -> bool {
    for rule in rules {
        if rule.patt.is_match(&dns_msg.ques[0].qname.join(".")) {
            //io::stdout().flush();

            dns_msg.head.qe = 256 | 0x8080;
            dns_msg.head.anc = 1;
            dns_msg.head.nsc = 0;
            dns_msg.head.arc = 0;
            //dns_msg.ques[0].qtype = 1;

            dns_msg.ansr.push(dns::RR{
                name: dns_msg.ques[0].qname.clone(),
                tp: 1,
                class: 1,
                ttl: 299,
                //ttl: 28800,
                rdlen: 4,
                rdata: dns::Rdata::IpAddr(rule.ip),
            });
            //dns::show_dns(&buf[..len + 16]);
            //println!("{:?}", dns::to_dns(&buf));
            println!("{} match rule {:?} ", &dns_msg.ques[0].qname.join("."), rule);
            return true;
        }
    }
    false
}

fn random_udp(ip: Ipv4Addr) -> UdpSocket {
    loop {
        let socket_addr =  SocketAddrV4::new(ip, ((rand::random::<u16>() % 16382) + 49152));
        match UdpSocket::bind(socket_addr){
            Ok(s) => {
                return s
            }
            _ => {
            }
        };
    };
}
