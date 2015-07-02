#![allow(unused_mut, unused_variables, unused_must_use)]
#![feature(append, test)]

use std::io::{Read, Cursor, Write};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::string::String;


#[derive(Debug, PartialEq)]
pub enum Rdata {
    Cname(Vec<String>),
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
}

impl std::default::Default for Rdata {
    fn default() -> Rdata {
       Rdata::Ipv4(Ipv4Addr::new(192, 168, 0, 1))
    }
}


#[derive(Default, Debug, PartialEq)]
pub struct Header {
    pub id: u16,
    pub qe: u16,
    pub qdc: u16,
    pub anc: u16,
    pub nsc: u16,
    pub arc: u16,
}

#[derive(Default, Debug, PartialEq)]
pub struct Question {
    pub qname: Vec<String>,
    pub qtype: u16,
    pub qclass: u16,
}

#[derive(Default, Debug, PartialEq)]
pub struct RR {
   pub name: Vec<String>,
   pub tp: u16,
   pub class: u16,
   pub ttl: i32,
   pub rdlen: u16,
   pub rdata: Rdata,
}

#[derive(Default, Debug, PartialEq)]
pub struct DnsMsg {
    pub head: Header,
    pub ques: Vec<Question>,
    pub ansr: Vec<RR>,
    pub auth: Vec<RR>,
    pub addi: Vec<RR>,
}

trait MyReadExt: Read {
    fn read_exact(&mut self, usize) -> Vec<u8>;

    fn read_u8(&mut self) -> u8 {
        let buf = self.read_exact(1);
        buf[0]
    }
    fn read_u16(&mut self) -> u16 {
        let buf = self.read_exact(2);
        ((buf[0] as u16) << 8) + (buf[1] as u16)
    }
    fn read_i32(&mut self) -> i32 {
        let buf = self.read_exact(4);
        ((buf[0] as i32) << 24) + ((buf[1] as i32) << 16) + ((buf[2] as i32) << 8) + (buf[3] as i32)
    }
}

impl<'a> MyReadExt for Cursor<&'a [u8]> {
    fn read_exact(&mut self, len: usize ) -> Vec<u8> {
        let mut buf = Vec::with_capacity(len);
        self.take(len as u64).read_to_end(&mut buf);
        buf
    }
}

trait MyWriteExt: Write {
    fn write_u16(&mut self, data: u16) {
        let buf = [(data >> 8) as u8, data as u8];
        self.write_all(&buf);
    }
    fn write_i32(&mut self, data: i32) {
        let buf = [(data >> 24) as u8, (data >> 16) as u8, (data >> 8) as u8, data as u8];
        self.write_all(&buf);
    }
}

impl<'a> MyWriteExt for Cursor<&'a mut [u8]> {}

//pub fn show_dns(buf: &[u8]) {
    //let len = buf.len();
        //println!("dns {}", len);
        //for i in (0..len-1).step_by(2) {
            //unsafe{
                //println!("{}-{}: {:0>8b} {:0>8b}: {:?}", i, i+1, &buf[i], &buf[i+1], str::from_utf8_unchecked(&buf[i..i+2]));
            //}
        //}
        //if len%2 != 0 {
            //unsafe{
                //println!("{}: {:0>8b}: {:?}", len-1, &buf[len - 1], str::from_utf8_unchecked(&buf[len - 1..len]));
            //}
        //}
//}

pub fn to_dns(buf: &[u8]) -> DnsMsg {
    let mut reader = Cursor::new(buf);
    let mut msg: DnsMsg=  std::default::Default::default();
    msg.head.id  = reader.read_u16();
    msg.head.qe  = reader.read_u16();
    msg.head.qdc = reader.read_u16();
    msg.head.anc = reader.read_u16();
    msg.head.nsc = reader.read_u16();
    msg.head.arc = reader.read_u16();
    for _ in (0..msg.head.qdc) {
       let mut q: Question = std::default::Default::default();
        q.qname  = decode_url(&mut reader);
        q.qtype  = reader.read_u16();
        q.qclass = reader.read_u16();
        msg.ques.push(q);
    }
    if msg.head.anc > 0 {
        //println!("have ansr");
        for _ in (0..msg.head.anc) {
            msg.ansr.push(to_rr(&mut reader));
        }
    }
    //if msg.head.nsc > 0 {
        //println!("have auth");
        //for _ in (0..msg.head.nsc) {
            //msg.auth.push(to_rr(&mut reader));
        //}
    //}
    //if msg.head.arc > 0 {
        //println!("have addi");
        //for _ in (0..msg.head.arc) {
            //msg.addi.push(to_rr(&mut reader));
        //}
    //}
    msg
}

pub fn to_rr(reader: &mut Cursor<&[u8]>) -> RR {
    let mut r: RR = std::default::Default::default();
    r.name  = decode_url(reader);
    r.tp    = reader.read_u16();
    r.class = reader.read_u16();
    r.ttl   = reader.read_i32();
    r.rdlen = reader.read_u16();
    match r.tp {
        1 => {
            r.rdata = Rdata::Ipv4(Ipv4Addr::new(
                    reader.read_u8(),
                    reader.read_u8(),
                    reader.read_u8(),
                    reader.read_u8(),
                    ));
        }
        28 => {
            r.rdata = Rdata::Ipv6(Ipv6Addr::new(
                    reader.read_u16(),
                    reader.read_u16(),
                    reader.read_u16(),
                    reader.read_u16(),
                    reader.read_u16(),
                    reader.read_u16(),
                    reader.read_u16(),
                    reader.read_u16(),
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

pub fn decode_url(reader: &mut Cursor<&[u8]>) -> Vec<String> {
    // 3www6google3com > www.google.com
    let mut j = reader.read_u8() as usize;
    //let mut s = String::with_capacity(63);
    let mut s: Vec<String> = vec!();
    loop {
        match j {
            1...64 => {
                s.push(String::from_utf8(reader.read_exact(j)).unwrap());
                j = reader.read_u8() as usize;
            }
            0 => {
                break;
            }
            _ => {
                let i = (((j ^ 0xC0) as u16) << 8) + (reader.read_u8() as u16);
                let b = reader.position();
                reader.set_position(i as u64);
                s.append(&mut decode_url(reader));
                reader.set_position(b);
                break;
            }
        }
    }
    s
}

pub fn from_dns(msg: &DnsMsg) -> [u8; 512] {
    let mut buf = [0u8; 512];
    {
        let mut writer = Cursor::new(&mut buf[..]);
        writer.write_u16(msg.head.id);
        writer.write_u16(msg.head.qe);
        writer.write_u16(msg.head.qdc);
        writer.write_u16(msg.head.anc);
        writer.write_u16(msg.head.nsc);
        writer.write_u16(msg.head.arc);
        for q in &msg.ques {
            for name in &q.qname {
                writer.write(&[name.len() as u8]);
                writer.write_fmt(format_args!("{}", name));
            }
            writer.write(&[0]);
            writer.write_u16(q.qtype);
            writer.write_u16(q.qclass);
        }
        for r in &msg.ansr {
            from_rr(&mut writer, r);
        }
    }
    buf
}

pub fn from_rr(writer: &mut Cursor<&mut [u8]>, r: &RR) {
    for name in &r.name {
        writer.write(&[name.len() as u8]);
        writer.write_fmt(format_args!("{}", name));
    }
    writer.write(&[0]);
    writer.write_u16(r.tp);
    writer.write_u16(r.class);
    writer.write_i32(r.ttl);
    writer.write_u16(r.rdlen);
    match &r.rdata {
        &Rdata::Ipv4(ip) => {
            writer.write(&ip.octets()[..]);
        }
        &Rdata::Cname(ref cname) => {
            for name in cname {
                writer.write(&[name.len() as u8]);
                writer.write_fmt(format_args!("{}", name));
            }
            writer.write(&[0]);
        }
        _ => {
        }
        //&Rdata::Ipv6(ip) => {
            //writer.write(&ip.octets()[..]);
        //}
    }
}

#[cfg(test)]
mod test {
    extern crate test;

    use super::*;
    use self::test::Bencher;

    #[bench]
    fn bench_to_dns(b: &mut Bencher) {
        let buf = [205, 228, 129, 128, 0, 1, 0, 3, 0, 0, 0, 0, 3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 5, 0, 1, 0, 0, 4, 82, 0, 15, 3, 119, 119, 119, 1, 97, 6, 115, 104, 105, 102, 101, 110, 192, 22, 192, 43, 0, 1, 0, 1, 0, 0, 0, 208, 0, 4, 119, 75, 218, 70, 192, 43, 0, 1, 0, 1, 0, 0, 0, 208, 0, 4, 119, 75, 217, 109];
        b.iter(||{
            to_dns(&buf);
        })
    }

    #[bench]
    fn bench_from_dns(b: &mut Bencher) {
        let buf = [205, 228, 129, 128, 0, 1, 0, 3, 0, 0, 0, 0, 3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 5, 0, 1, 0, 0, 4, 82, 0, 15, 3, 119, 119, 119, 1, 97, 6, 115, 104, 105, 102, 101, 110, 192, 22, 192, 43, 0, 1, 0, 1, 0, 0, 0, 208, 0, 4, 119, 75, 218, 70, 192, 43, 0, 1, 0, 1, 0, 0, 0, 208, 0, 4, 119, 75, 217, 109];
        let msg = to_dns(&buf);
        b.iter(||{
            from_dns(&msg);
        })
    }

    #[test]
    fn test_to_dns() {
        let buf = [205, 228, 129, 128, 0, 1, 0, 3, 0, 0, 0, 0, 3, 119, 119, 119, 5, 98, 97, 105, 100, 117, 3, 99, 111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 5, 0, 1, 0, 0, 4, 82, 0, 15, 3, 119, 119, 119, 1, 97, 6, 115, 104, 105, 102, 101, 110, 192, 22, 192, 43, 0, 1, 0, 1, 0, 0, 0, 208, 0, 4, 119, 75, 218, 70, 192, 43, 0, 1, 0, 1, 0, 0, 0, 208, 0, 4, 119, 75, 217, 109];
        assert_eq!(
            format!("{:?}", to_dns(&buf)),
            r#"DnsMsg { head: Header { id: 52708, qe: 33152, qdc: 1, anc: 3, nsc: 0, arc: 0 }, ques: [Question { qname: ["www", "baidu", "com"], qtype: 1, qclass: 1 }], ansr: [RR { name: ["www", "baidu", "com"], tp: 5, class: 1, ttl: 1106, rdlen: 15, rdata: Cname(["www", "a", "shifen", "com"]) }, RR { name: ["www", "a", "shifen", "com"], tp: 1, class: 1, ttl: 208, rdlen: 4, rdata: Ipv4(119.75.218.70) }, RR { name: ["www", "a", "shifen", "com"], tp: 1, class: 1, ttl: 208, rdlen: 4, rdata: Ipv4(119.75.217.109) }], auth: [], addi: [] }"# 
            );
    }
}
