extern crate std;

use std::old_io::{TcpListener, TcpStream, Acceptor, Listener, timer};
use std::old_io::{BufReader, BufWriter};
use std::old_io::net::udp::UdpSocket;
use std::thread::Thread;
use std::old_io::net::ip::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use std::str;
use regex::Regex;
use std::str::FromStr;


#[derive(Debug)]
pub enum Rdata {
    Cname(Vec<String>),
    Ip(IpAddr),
}

#[derive(Debug)]
pub struct Label(Vec<String>);


#[derive(Default, Debug)]
pub struct Header {
    pub id  : Vec<u8>, // u16
    pub qe  : Vec<u8>, // u16
    pub qdc : Vec<u8>, // u16
    pub anc : Vec<u8>, // u16
    pub nsc : Vec<u8>, // u16
    pub arc : Vec<u8>, // u16
}

#[derive(Default, Debug)]
pub struct Question {
    pub qname  : Vec<u8>, // label
    pub qtype  : Vec<u8>, // u16
    pub qclass : Vec<u8>, // u16
}

#[derive(Default, Debug)]
pub struct RR {
   pub name  : Vec<u8>, // label
   pub tp    : Vec<u8>, // u16
   pub class : Vec<u8>, // u16
   pub ttl   : Vec<u8>, // i32
   pub rdlen : Vec<u8>, // u16
   pub rdata : Vec<u8>, // Rdata
}

#[derive(Default, Debug)]
pub struct DnsMsg {
    pub head: Header,
    pub ques: Vec<Question>,
    pub ansr: Vec<RR>,
    pub auth: Vec<RR>,
    pub addi: Vec<RR>,
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

pub fn as_dns(buf: &[u8]) -> DnsMsg {
    let mut msg: DnsMsg = std::default::Default::default();
    let mut reader = BufReader::new(buf);

    msg.head.id  = reader.read_exact(2).unwrap();
    msg.head.qe  = reader.read_exact(2).unwrap();
    msg.head.qdc = reader.read_exact(2).unwrap();
    msg.head.anc = reader.read_exact(2).unwrap();
    msg.head.nsc = reader.read_exact(2).unwrap();
    msg.head.arc = reader.read_exact(2).unwrap();

    for _ in range(0, as_u16(&(msg.head.qdc))) {
        let mut q: Question = std::default::Default::default();
        q.qname = get_label(&mut reader);
        q.qtype   = reader.read_exact(2).unwrap();
        q.qclass  = reader.read_exact(2).unwrap();
        msg.ques.push(q);
    }

    for _ in range(0, as_u16(&(msg.head.anc))) {
        msg.ansr.push(to_rr(&mut reader));
    }

    for _ in range(0, as_u16(&(msg.head.nsc))) {
        msg.auth.push(to_rr(&mut reader));
    }

    for _ in range(0, as_u16(&(msg.head.arc))) {
        msg.addi.push(to_rr(&mut reader));
    }

    msg
}

pub fn as_slice(msg: &mut DnsMsg) -> Vec<u8> {
    let mut v: Vec<u8> = vec!();
    v.push_all(&msg.head.id);
    v.push_all(&msg.head.qe);
    v.push_all(&msg.head.qdc);
    v.push_all(&msg.head.anc);
    v.push_all(&msg.head.nsc);
    v.push_all(&msg.head.arc);

    for q in msg.ques.iter() {
        v.push_all(&q.qname);
        v.push_all(&q.qtype);
        v.push_all(&q.qclass);
    }

    for r in msg.ansr.iter() {
        v.push_all(&r.name);
        v.push_all(&r.tp);
        v.push_all(&r.class);
        v.push_all(&r.ttl);
        v.push_all(&r.rdlen);
        v.push_all(&r.rdata);
    }

    for r in msg.auth.iter() {
        v.push_all(&r.name);
        v.push_all(&r.tp);
        v.push_all(&r.class);
        v.push_all(&r.ttl);
        v.push_all(&r.rdlen);
        v.push_all(&r.rdata);
    }

    for r in msg.addi.iter() {
        v.push_all(&r.name);
        v.push_all(&r.tp);
        v.push_all(&r.class);
        v.push_all(&r.ttl);
        v.push_all(&r.rdlen);
        v.push_all(&r.rdata);
    }
    v
}

fn to_rr(reader: &mut BufReader) -> RR {
    let mut r: RR = std::default::Default::default();
    r.name  = get_label(reader);
    r.tp    = reader.read_exact(2).unwrap();
    r.class = reader.read_exact(2).unwrap();
    r.ttl   = reader.read_exact(4).unwrap();
    r.rdlen = reader.read_exact(2).unwrap();
    r.rdata = reader.read_exact(as_u16(&(r.rdlen))).unwrap();
    //match r.tp {
        //1 => {
            //r.rdata = Rdata::Ip(Ipv4Addr(
                    //reader.read_u8().unwrap(),
                    //reader.read_u8().unwrap(),
                    //reader.read_u8().unwrap(),
                    //reader.read_u8().unwrap(),
                    //));
        //}
        //5 => {
            //r.rdata = Rdata::Cname(decode_url(reader));
        //}
        //_ => {
            //panic!("unmatched type");
        //}
    //}
    r
}

fn as_u16(v: &Vec<u8>) -> usize {
    (((v[0] as u16) << 8) + (v[1] as u16)) as usize

}

//fn get_label(reader: &mut BufReader, save: HashMap<String, usize>) -> Vec<u8> {
fn get_label(reader: &mut BufReader) -> Vec<u8> {
    // 3www6google3com > www.google.com
    let mut s: Vec<u8> = vec!();
    loop {
        let j = reader.read_u8().unwrap() as usize;
        s.push(j as u8);
        match j {
            1...64 => {
                s.push_all(&reader.read_exact(j).unwrap());
                //save.push(s[s.len()-j..j], reader.tell().unwrap())
            }
            0 => {
                break;
            }
            _  => {
                s.push_all(&reader.read_exact(1).unwrap());
                break;
            }
        }
    }

    s
}
