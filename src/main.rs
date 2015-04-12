#![allow(unused_imports, unused_mut, unused_variables, unused_must_use, unused_features, dead_code)]
#![feature(udp, collections, step_by)]

extern crate regex;
extern crate rand;

mod dns;

use std::io::{BufReader, BufRead, Write, Cursor};
use std::fs::File;
use std::net::UdpSocket;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::thread::Thread;
use std::str;
use regex::Regex;
use std::str::FromStr;


fn main() {
    let rules = parse_rule();
    let local = "127.0.0.1:53";
    let up_dns = "8.8.8.8:53";
    let mut local_socket = UdpSocket::bind(local).unwrap();
    let mut buf = [0u8; 512];
    'outer: loop {
        //let mut local_socket1 = local_socket.clone();
        //Thread::spawn(|| {
            //print!("wait for a requese ... ");
            match local_socket.recv_from(&mut buf){
                Ok((len, src)) => {


                    //dns::show_dns(&buf[..len]);
                    let mut dns_msg = dns::to_dns(&buf);
                    //println!("{:?}", dns_msg);
                    print!("recvice a requese for {} ", &dns_msg.ques[0].qname.connect("."));

                    for rule in &rules {
                        if rule.patt.is_match(&dns_msg.ques[0].qname.connect(".")) {
                            print!("matched rule {:?}", rule);

                            dns_msg.head.qe = dns_msg.head.qe | 0x8080;
                            dns_msg.head.anc = 1;
                            dns_msg.head.nsc = 0;
                            dns_msg.head.arc = 0;
                            //dns_msg.ques[0].qtype = 1;

                            dns_msg.ansr.push(dns::RR{
                                name: dns_msg.ques[0].qname.clone(),
                                tp: 1,
                                class: 1,
                                ttl: 200,
                                rdlen: 4,
                                rdata: dns::Rdata::Ipv4(rule.ip),
                            });
                            buf = dns::from_dns(&dns_msg);
                            local_socket.send_to(&buf[..], src);
                            //dns::show_dns(&buf[..len + 16]);
                            //println!("{:?}", dns::to_dns(&buf));
                            println!(" ... dns response finished");
                            continue 'outer;
                        }
                    }


                    let mut dns_socket = random_udp(Ipv4Addr::new(0, 0, 0, 0));
                    dns_socket.set_time_to_live(300);

                    dns_socket.send_to(&buf[..len], up_dns);

                    match dns_socket.recv_from(&mut buf){
                        Ok((len, _)) => {
                            local_socket.send_to(&buf[..len], src);

                            //dns::show_dns(&buf[..len]);
                            //println!("{:?}", &buf[..len]);
                            //println!("{:?}", dns::to_dns(&buf));
                            println!(" ... dns response finished");
                        },
                        Err(e) => {
                            println!(" {}",e);
                        },
                    };
                }
                Err(e) => {
                    println!("An err: {}",e);
                    //unreachable!()
                }
            };
        //});
    }
}

#[derive(Debug)]
struct Rule {
    ip: Ipv4Addr,
    patt: Regex,
}

fn parse_rule() -> Vec<Rule> {
    let mut rules: Vec<Rule> = Vec::new();
    let gm = Regex::new(r"#\$ *([^ ]*) *([^ ]*)").unwrap();
    for line in BufReader::new(File::open("/etc/hosts").unwrap()).lines() {
        let l = line.as_ref().unwrap();
        if l.starts_with("#$") {
            let cap = gm.captures(l).unwrap();
            rules.push(Rule{ip: FromStr::from_str(cap.at(1).unwrap()).unwrap(), patt: Regex::new(cap.at(2).unwrap()).unwrap()});
        }
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
