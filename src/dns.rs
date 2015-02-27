extern crate std;
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

#[derive(Debug)]
pub struct Rule {
    pub ip: IpAddr,
    pub patt: Regex,
}


#[derive(Debug)]
pub enum Rdata {
    Cname(Vec<String>),
    Ip(IpAddr),
}

impl std::default::Default for Rdata {
    fn default() -> Rdata {
       Rdata::Ip(Ipv4Addr(192, 168, 0, 1))
    }
}


#[derive(Default, Debug)]
pub struct Header {
    pub id: u16,
    pub qe: u16,
    pub qdc: u16,
    pub anc: u16,
    pub nsc: u16,
    pub arc: u16,
}

#[derive(Default, Debug)]
pub struct Question {
    pub qname: Vec<String>,
    pub qtype: u16,
    pub qclass: u16,
}

#[derive(Default, Debug)]
pub struct RR {
   pub name: Vec<String>,
   pub tp: u16,
   pub class: u16,
   pub ttl: i32,
   pub rdlen: u16,
   pub rdata: Rdata,
}

#[derive(Default, Debug)]
pub struct DnsMsg {
    pub head: Header,
    pub ques: Vec<Question>,
    pub ansr: Vec<RR>,
    pub auth: Vec<RR>,
    pub addi: Vec<RR>,
}


pub fn parse_rule() -> Vec<Rule>{
    let mut rules: Vec<Rule> = Vec::new();
    for line in BufferedReader::new(File::open(&Path::new("/etc/hosts"))).lines() {
        if line.clone().unwrap().starts_with("#$") {
            let l = (line.clone().unwrap()).trim_right_matches('\n').trim_left_matches('#').trim_left_matches('$').trim().split(' ').map(|s| s.to_string()).fold(Vec::new(), |mut a, b| { a.push(b); a});
            rules.push(Rule{ip: FromStr::from_str(&l[0]).unwrap(), patt: Regex::new(&l[1]).unwrap()});
        }
    }
    rules
}

pub fn random_udp(ip: IpAddr) -> UdpSocket {
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

pub fn show_dns(buf: &[u8]) {
    let len = buf.len();
        println!("dns {}", len);
        for i in std::iter::range_step(0, len-1, 2) {
            unsafe{
                println!("{}-{}: {:0>8b} {:0>8b}: {:?}", i, i+1, &buf[i], &buf[i+1], str::from_utf8_unchecked(&buf[i..i+2]));
            }
        }
        if len%2 != 0 {
            unsafe{
                println!("{}: {:0>8b}: {:?}", len-1, &buf[len - 1], str::from_utf8_unchecked(&buf[len - 1..len]));
            }
        }
}

pub fn to_dns(buf: &[u8]) -> DnsMsg {
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

pub fn to_rr(reader: &mut BufReader) -> RR {
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

pub fn decode_url(reader: &mut BufReader) -> Vec<String> {
    // 3www6google3com > www.google.com
    let mut j = reader.read_u8().unwrap() as usize;
    //let mut s = String::with_capacity(63);
    let mut s: Vec<String> = vec!();
    loop {
        match j {
            1...64 => {
                s.push(std::string::String::from_utf8((reader.read_exact(j).unwrap())).unwrap());
                j = reader.read_u8().unwrap() as usize;
            }
            0 => {
                break;
            }
            _  => {
                reader.seek(-1, std::old_io::SeekStyle::SeekCur);
                let i = (reader.read_be_u16().unwrap() ^ 0xC000) as usize;
                let b = reader.tell().unwrap();
                reader.seek(i as i64, std::old_io::SeekStyle::SeekSet);
                s.append(&mut decode_url(reader));
                reader.seek(b as i64, std::old_io::SeekStyle::SeekSet);
                break;
            }
        }
    }
    s
}

