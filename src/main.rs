#![allow(unused_imports, unused_mut, unused_variables, unused_must_use, unused_features, dead_code)]
#![feature(io, std_misc, core)]

extern crate rand;
extern crate regex;

use std::old_io::{TcpListener, TcpStream, Acceptor, Listener, timer};
use std::old_io::{BufReader, BufWriter, BufferedReader, File};
use std::old_io::net::udp::UdpSocket;
use std::thread::Thread;
use std::old_io::net::ip::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use std::str;
use regex::Regex;
use std::str::FromStr;

#[derive(Show)]
struct Rule {
    ip: IpAddr,
    patt: Regex,
}


#[derive(Show)]
enum Rdata {
    Cname(String),
    Ip(IpAddr),
}

impl std::default::Default for Rdata {
    fn default() -> Rdata {
       Rdata::Cname(" ".to_string())
    }
}


#[derive(Default, Show)]
struct Header {
    id: u16,
    qe: u16,
    qdc: u16,
    anc: u16,
    nsc: u16,
    arc: u16,
}

#[derive(Default, Show)]
struct Question {
    qname: String,
    qtype: u16,
    qclass: u16,
}

#[derive(Default, Show)]
struct RR {
    name: String,
    tp: u16,
    class: u16,
    ttl: i32,
    rdlen: u16,
    rdata: Rdata,
}

#[derive(Default, Show)]
struct DnsMsg {
    head: Header,
    ques: Vec<Question>,
    ansr: Vec<RR>,
    auth: Vec<RR>,
    addi: Vec<RR>,
}

fn main() {
    let rules = parse_rule();
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

                    let mut dns_socket = random_udp(Ipv4Addr(0, 0, 0, 0));

                    show_dns(&buf[..len]);
                    let dns_msg = to_dns(&buf);
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
                            show_dns(&buf[..len + 16]);
                            println!("{:?}", to_dns(&buf));
                            println!(" ... dns response finished");
                        } else {
                            dns_socket.set_timeout(Some(300));
                            dns_socket.send_to(&buf[..len], up_dns);

                            match dns_socket.recv_from(&mut buf){
                                Ok((len, _)) => {
                                    local_socket.send_to(&buf[..len], src);

                                    show_dns(&buf[..len]);
                                    println!("{:?}", to_dns(&buf));
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

fn parse_rule() -> Vec<Rule>{
    let mut rules: Vec<Rule> = Vec::new();
    for line in BufferedReader::new(File::open(&Path::new("/etc/hosts"))).lines() {
        if line.clone().unwrap().starts_with("#$") {
            let l = (line.clone().unwrap()).trim_right_matches('\n').trim_left_matches('#').trim_left_matches('$').trim().split(' ').map(|s| s.to_string()).fold(Vec::new(), |mut a, b| { a.push(b); a});
            rules.push(Rule{ip: FromStr::from_str(&l[0]).unwrap(), patt: Regex::new(&l[1]).unwrap()});
        }
    }
    rules
}

fn random_udp(ip: IpAddr) -> UdpSocket {
    loop {
        let socket_addr =  SocketAddr { ip: ip, port: ((rand::random::<u16>() % 16382) + 49152) };
        match UdpSocket::bind(socket_addr){
            Ok(s) => {
                return s
            }
            _ => {
            }
        };
    };
}

fn show_dns(buf: &[u8]) {
    let len = buf.len();
    unsafe{
        println!("dns {}", len);
        for i in std::iter::range_step(0, len-1, 2) {
            println!("{}-{}: {:0>8b} {:0>8b}: {:?}", i, i+1, &buf[i], &buf[i+1], str::from_utf8_unchecked(&buf[i..i+2]));
        }
        if len%2 != 0 {
            println!("{}: {:0>8b}: {:?}", len-1, &buf[len - 1], str::from_utf8_unchecked(&buf[len - 1..len]));
        }
    }
}

fn to_dns(buf: &[u8]) -> DnsMsg {
    let mut reader = BufReader::new(buf);
    let mut msg: DnsMsg=  std::default::Default::default();
    msg.head.id  = reader.read_be_u16().unwrap();
    msg.head.qe  = reader.read_be_u16().unwrap();
    msg.head.qdc = reader.read_be_u16().unwrap();
    msg.head.anc = reader.read_be_u16().unwrap();
    msg.head.nsc = reader.read_be_u16().unwrap();
    msg.head.arc = reader.read_be_u16().unwrap();
    for _ in range(0, msg.head.qdc) {
       let mut q: Question = std::default::Default::default();
        q.qname  = decode_url(&mut reader);
        q.qtype  = reader.read_be_u16().unwrap();
        q.qclass = reader.read_be_u16().unwrap();
        msg.ques.push(q);
    }
    if msg.head.anc > 0 {
        println!("have ansr");
        for _ in range(0, msg.head.anc) {
            msg.ansr.push(to_rr(&mut reader));
        }
    }
    //if msg.head.nsc > 0 {
        //println!("have auth");
        //for _ in range(0, msg.head.nsc) {
            //msg.auth.push(to_rr(&mut reader));
        //}
    //}
    //if msg.head.arc > 0 {
        //println!("have addi");
        //for _ in range(0, msg.head.arc) {
            //msg.addi.push(to_rr(&mut reader));
        //}
    //}
    msg
}

fn to_rr(reader: &mut BufReader) -> RR {
    let mut r: RR = std::default::Default::default();
    r.name  = decode_url(reader);
    r.tp    = reader.read_be_u16().unwrap();
    r.class = reader.read_be_u16().unwrap();
    r.ttl   = reader.read_be_i32().unwrap();
    r.rdlen = reader.read_be_u16().unwrap();
    match r.tp {
        1 => {
            r.rdata = Rdata::Ip(Ipv4Addr(
                    reader.read_u8().unwrap(),
                    reader.read_u8().unwrap(),
                    reader.read_u8().unwrap(),
                    reader.read_u8().unwrap(),
                    ));
        }
        5 => {
            r.rdata = Rdata::Cname(decode_url(reader));
        }
        _ => {
            panic!("unmatched type");
        }
    }
    r
}

fn decode_url(reader: &mut BufReader) -> String {
    // 3www6google3com > www.google.com
    let mut j = reader.read_u8().unwrap() as usize;
    let mut s = String::with_capacity(63);
    loop {
        match j {
            1...64 => {
                s.push_str(str::from_utf8(&(reader.read_exact(j).unwrap())).unwrap());
                s.push_str(".");
                j = reader.read_u8().unwrap() as usize;
            }
            0 => {
                s.pop();
                break;
            }
            _  => {
                reader.seek(-1, std::old_io::SeekStyle::SeekCur);
                let i = (reader.read_be_u16().unwrap() ^ 0xC000) as usize;
                let b = reader.tell().unwrap();
                reader.seek(i as i64, std::old_io::SeekStyle::SeekSet);
                s.push_str(&decode_url(reader));
                reader.seek(b as i64, std::old_io::SeekStyle::SeekSet);
                break;
            }
        }
    }
    s
}

