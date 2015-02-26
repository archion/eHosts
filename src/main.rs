#![allow(unused_imports, unused_mut, unused_variables, unused_must_use, unused_features, dead_code)]
#![feature(io, old_io, old_path, std_misc, core)]

extern crate regex;

mod dns;

use std::old_io::{TcpListener, TcpStream, Acceptor, Listener, timer};
use std::old_io::{BufReader, BufWriter, BufferedReader, File};
use std::old_io::net::udp::UdpSocket;
use std::thread::Thread;
use std::old_io::net::ip::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use std::str;
use regex::Regex;
use std::str::FromStr;


fn main() {
    let rules = dns::parse_rule();
    let local = SocketAddr { ip: Ipv4Addr(127, 0, 0, 1), port: 53 };
    let up_dns = SocketAddr { ip: Ipv4Addr(8, 8, 8, 8), port: 53 };
    let mut local_socket = UdpSocket::bind(local).unwrap();
    loop {
        //let mut local_socket1 = local_socket.clone();
        //Thread::spawn(|| {
            print!("wait for a requese ... ");
            let mut buf = [0u8; 512];
            match local_socket.recv_from(&mut buf){
                Ok((len, src)) => {
                    print!("recvice a local requese at {:20}\n", src);

                    let mut dns_socket = dns::random_udp(Ipv4Addr(0, 0, 0, 0));

                    dns::show_dns(&buf[..len]);
                    let dns_msg = dns::to_dns(&buf);
                    println!("{:?}", dns_msg);

                    for rule in rules.iter() {
                        if rule.patt.is_match(&dns_msg.ques[0].qname) {
                            println!("match {} for {}", dns_msg.ques[0].qname, rule.ip );
                            let (i1, i2, i3, i4) = match rule.ip{
                                Ipv4Addr(i1, i2, i3, i4) => (i1, i2, i3, i4),
                                _ => {
                                    unreachable!()
                                }
                            };
                            {
                                let mut resp = BufWriter::new(&mut buf);
                                resp.seek(2 as i64, std::old_io::SeekStyle::SeekSet);
                                resp.write_all(&[129, 128, 0, 1, 0, 1]);
                                resp.seek(len as i64, std::old_io::SeekStyle::SeekSet);
                                resp.write_all(&[192, 12, 0, 1, 0, 1, 0, 0, 0, 0, 0, 4, i1, i2, i3, i4]);
                            }
                            local_socket.send_to(&buf[..len + 16], src);
                            dns::show_dns(&buf[..len + 16]);
                            println!("{:?}", dns::to_dns(&buf));
                            println!(" ... dns response finished");
                        } else {
                            dns_socket.set_timeout(Some(300));
                            dns_socket.send_to(&buf[..len], up_dns);

                            match dns_socket.recv_from(&mut buf){
                                Ok((len, _)) => {
                                    local_socket.send_to(&buf[..len], src);

                                    dns::show_dns(&buf[..len]);
                                    println!("{:?}", dns::to_dns(&buf));
                                    println!(" ... dns response finished");
                                },
                                Err(e) => {
                                    println!(" {}",e);
                                },
                            };
                        }
                        //timer::sleep(Duration::seconds(10));
                    }
                }
                Err(e) => {
                    println!("a Err {}",e);
                    //unreachable!()
                }
            };
        //});
    }
}
