#![allow(unused_mut, unused_variables, unused_must_use)]
#![feature(socket_timeout, duration)]

extern crate regex;
extern crate rand;
extern crate clap;
extern crate dns;

use regex::Regex;
use clap::{Arg, App};
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, SeekFrom};
use std::fs::File;
use std::net::{UdpSocket, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::thread;
use std::str::FromStr;
use std::time::Duration;
use std::fs;
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
        .get_matches();

    let local =  if matches.is_present("mode") {
        println!("Run eHost in servers mode"); 
        "0.0.0.0:53"
    }else{
        println!("Run eHost in local mode"); 
        "127.0.0.1:53"
    };

    let up_dns: Vec<SocketAddr> = matches.values_of("addr").map_or(vec![FromStr::from_str("8.8.8.8:53").unwrap()], |s| {
        s.iter().map(|a| {
            if a.contains(":") {
                FromStr::from_str(a).unwrap()
            }else{
                FromStr::from_str(format!("{}:53", a).as_ref()).unwrap()  
            }
        }).collect()
    });
    
    println!("Upstream DNS is {:?}", up_dns);

    let path = matches.value_of("file").unwrap_or("./hosts");
    println!("The hosts file is '{}'", path);
    let mut file = match File::open(&path) {
        Ok(file) => {
            file
        }
        Err(_) => {
            //print!("Hosts file doesn't exit, use /etc/hosts instead");
            //File::open("/etc/hosts").unwrap()
            print!("File '{}' doesn't exit! Please specify a rules file via '-f' option", path);
            return
        }
    };
    let mut rules = parse_rule(&file);
    if rules.len() == 0 {
        println!("Warn: file '{}' doesn't contain any rules!", path);
    }

    let mut mtime = file.metadata().unwrap().mtime();

    if cfg!(windows) {
        println!("auto set dns is not support in Windows, please set dns manually!");
    }else{
        set_dns();
    }
    let mut local_socket = UdpSocket::bind(local).unwrap();
    let mut buf = [0u8; 512];
    loop {
        //let mut local_socket1 = local_socket.clone();
        //print!("wait ... ");
        //io::stdout().flush();
        match local_socket.recv_from(&mut buf){
            Ok((len, src)) => {
                if mtime != file.metadata().unwrap().mtime() {
                    mtime = file.metadata().unwrap().mtime();
                    file.seek(SeekFrom::Start(0));
                    rules = parse_rule(&file);
                    println!("rules updated");
                }
                let local_socket = local_socket.try_clone().unwrap();
                let rules = rules.clone();
                let up_dns = up_dns.clone();
                thread::spawn(move || {
                    //dns::show_dns(&buf[..len]);
                    let mut dns_msg = dns::to_dns(&buf);
                    //println!("{:?}", dns_msg);
                    //io::stdout().flush();

                    //thread::sleep_ms(10000);

                    for rule in &rules {
                        if rule.patt.is_match(&dns_msg.ques[0].qname.connect(".")) {
                            //io::stdout().flush();

                            dns_msg.head.qe = dns_msg.head.qe | 0x8080;
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
                                rdata: dns::Rdata::Ipv4(rule.ip),
                            });
                            buf = dns::from_dns(&dns_msg);
                            local_socket.send_to(&buf[..], src);
                            //dns::show_dns(&buf[..len + 16]);
                            //println!("{:?}", dns::to_dns(&buf));
                            println!("{} match rule {:?} ", &dns_msg.ques[0].qname.connect("."), rule);
                            return;
                        }
                    }

                    let mut dns_socket = random_udp(Ipv4Addr::new(0, 0, 0, 0));

                    //set timeout
                    //dns_socket.set_read_timeout(Some(Duration::from_millis(500)));
                    dns_socket.set_read_timeout(Some(Duration::from_millis(500)));

                    for up_dns in up_dns {
                        dns_socket.send_to(&buf[..len], up_dns);

                        match dns_socket.recv_from(&mut buf){
                            Ok((len, _)) => {
                                local_socket.send_to(&buf[..len], src);

                                //dns::show_dns(&buf[..len]);
                                //println!("{:?}", &buf[..len]);
                                //println!("{:?}", dns::to_dns(&buf));
                                //drop(dns_socket);
                                println!("{} doesn't match any rules, response from {}", &dns_msg.ques[0].qname.connect("."), up_dns);
                                break
                            },
                            Err(e) => {
                                println!("{} dns {} timeout {}", &dns_msg.ques[0].qname.connect("."), up_dns, e);
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
    }
}

#[cfg(windows)]
trait MyMtime {
    fn mtime(&self) -> u64;
}

#[cfg(windows)]
impl MyMtime for fs::Metadata {
    fn mtime(&self) -> u64 {
        self.last_write_time()
    }
}

#[derive(Debug, Clone)]
struct Rule {
    ip: Ipv4Addr,
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
    println!("auto changing dns setting to 127.0.0.1")
}

fn parse_rule(file: &File) -> Vec<Rule> {
    let mut rules: Vec<Rule> = Vec::new();
    let gm = Regex::new(r"#\$ *([^ ]*) *([^ ]*)").unwrap();

    let mut buf = String::new();
    BufReader::new(file).read_to_string(&mut buf);

    for l in buf.lines_any() {
        //let l = line.as_ref().unwrap();
        if l.starts_with("#$") {
            let cap = gm.captures(l).unwrap();
            rules.push(Rule{ip: FromStr::from_str(cap.at(1).unwrap()).unwrap(), patt: Regex::new(cap.at(2).unwrap()).unwrap()});
        }
    }

    println!(", The Rules is");
    for r in &rules {
        println!("{:?}", r);
    }

    rules
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
