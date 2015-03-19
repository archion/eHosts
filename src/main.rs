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
    let local = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 53);
    let up_dns = SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 53);
    let mut local_socket = UdpSocket::bind(local).unwrap();
    loop {
        //let mut local_socket1 = local_socket.clone();
        //Thread::spawn(|| {
            print!("wait for a requese ... ");
            let mut buf = [0u8; 512];
            match local_socket.recv_from(&mut buf){
                Ok((len, src)) => {
                    print!("recvice a local requese at {:20}\n", src);


                    dns::show_dns(&buf[..len]);
                    let dns_msg = dns::to_dns(&buf);
                    println!("{:?}", dns_msg);

                    for rule in rules.iter() {
                        if rule.patt.is_match(&dns_msg.ques[0].qname.connect(".")) {
                            println!("match {:?} for {}", dns_msg.ques[0].qname.connect("."), rule.ip );
                            {
                                let mut resp = Cursor::new(&mut buf[..]);
                                resp.set_position(2);
                                resp.write_all(&[129, 128, 0, 1, 0, 1]);
                                resp.set_position(len as u64);
                                resp.write_all(&[192, 12, 0, 1, 0, 1, 0, 0, 0, 0, 0, 4]);
                                resp.write_all(&(rule.ip.octets()));
                            }
                            local_socket.send_to(&buf[..len + 16], src);
                            dns::show_dns(&buf[..len + 16]);
                            println!("{:?}", dns::to_dns(&buf));
                            println!(" ... dns response finished");
                            continue;
                        }
                    }

                    let mut dns_socket = random_udp(Ipv4Addr::new(0, 0, 0, 0));
                    dns_socket.set_time_to_live(300);

                    dns_socket.send_to(&buf[..len], up_dns);

                    match dns_socket.recv_from(&mut buf){
                        Ok((len, _)) => {
                            local_socket.send_to(&buf[..len], src);

                            dns::show_dns(&buf[..len]);
                            println!("{:?}", &buf[..len]);
                            println!("{:?}", dns::to_dns(&buf));
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
    for line in BufReader::new(File::open("/etc/hosts").unwrap()).lines() {
        if line.clone().unwrap().starts_with("#$") {
            let l = (line.clone().unwrap()).trim_right_matches('\n').trim_left_matches('#').trim_left_matches('$').trim().split(' ').map(|s| s.to_string()).fold(Vec::new(), |mut a, b| { a.push(b); a});
            rules.push(Rule{ip: FromStr::from_str(&l[0]).unwrap(), patt: Regex::new(&l[1]).unwrap()});
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
