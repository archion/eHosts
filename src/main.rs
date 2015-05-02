#![allow(unused_imports, unused_mut, unused_variables, unused_must_use, unused_features, dead_code, deprecated)]
#![feature(udp, collections, step_by, test, libc, core, fs_time)]

extern crate regex;
extern crate rand;
extern crate libc;
extern crate getopts;

mod dns;

use regex::Regex;
use getopts::Options;
use std::env;
use std::io;
use std::io::{Read, BufReader, BufRead, BufWriter, Write, Cursor, SeekFrom, Seek};
use std::fs::Metadata;
use std::fs::File;
use std::net::UdpSocket;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::thread;
use std::str::FromStr;
#[cfg(not(windows))]
use std::os::unix::io::AsRawFd;
#[cfg(windows)]
use std::os::windows::io::AsRawSocket;
use libc::{c_void, timeval, setsockopt, SOL_SOCKET};
use libc::consts::os::bsd44::SO_RCVTIMEO;


fn main() {
    let mut opts = Options::new();
    opts.optopt("d", "", "set upstream DNS server, default is 8.8.8.8", "ip-address");
    opts.optopt("f", "", "set rule file path, default is ./hosts", "file-path");
    opts.optflag("s", "", "run in server mode");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&env::args().collect::<Vec<String>>()[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };

    if matches.opt_present("h") {
        print!("{}", opts.usage("Usage: eHosts <Options>"));
        return;
    }

    let mut local = "127.0.0.1:53";
    if matches.opt_present("s") {
        local = "0.0.0.0:53";
        println!("Run eHost in servers mode");
    }

    let up_dns : SocketAddr = FromStr::from_str(format!("{}:53", matches.opt_str("d").unwrap_or("8.8.8.8".to_string())).as_ref()).unwrap();
    println!("Upstream DNS is {}", up_dns);

    let path = matches.opt_str("f").unwrap_or("hosts".to_string());
    println!("The hosts file is '{}'", path);
    let mut file = match File::open(&path) {
        Ok(file) => {
            file
        }
        Err(_) => {
            //print!("Hosts file doesn't exit, use /etc/hosts instead");
            //File::open("/etc/hosts").unwrap()
            print!("File '{}' doesn't exit!", path);
            return
        }
    };
    let mut rules = parse_rule(&file);
    if rules.len() == 0 {
        println!("File '{}' doesn't contain any rules, exit!", path);
        return
    }

    let mut mtime = file.metadata().unwrap().modified();

    if cfg!(windows) {
        println!("auto set dns is not support in your OS, please set dns manually!");
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
                if mtime != file.metadata().unwrap().modified() {
                    mtime = file.metadata().unwrap().modified();
                    file.seek(SeekFrom::Start(0));
                    rules = parse_rule(&file);
                    println!("update rules");
                }
                let local_socket = local_socket.try_clone().unwrap();
                let rules = rules.clone();
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
                                ttl: 200,
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
                    dns_socket.set_timeout(3);

                    dns_socket.send_to(&buf[..len], up_dns);

                    match dns_socket.recv_from(&mut buf){
                        Ok((len, _)) => {
                            local_socket.send_to(&buf[..len], src);

                            //dns::show_dns(&buf[..len]);
                            //println!("{:?}", &buf[..len]);
                            //println!("{:?}", dns::to_dns(&buf));
                            //drop(dns_socket);
                            println!("{} doesn't match any rules", &dns_msg.ques[0].qname.connect("."));
                        },
                        Err(e) => {
                            println!("{} dns {} timeout {}", &dns_msg.ques[0].qname.connect("."), up_dns, e);
                        },
                    };
                });
            }
            Err(e) => {
                println!("An err: {}",e);
                //unreachable!()
            }
        };
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
        i+=1;
    }
    if i==1 {
        writer.write_fmt(format_args!("{}\n", "nameserver 8.8.8.8"));
    }
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

trait Timeout {
    fn set_timeout(&self, sec: i32);
}

impl Timeout for UdpSocket {
    #[cfg(not(windows))]
    fn set_timeout(&self, sec: i32){
        unsafe {
            setsockopt(self.as_raw_fd(), SOL_SOCKET, SO_RCVTIMEO, &timeval{tv_sec: sec, tv_usec: 0} as *const _ as *const c_void, std::mem::size_of::<timeval>() as u32);
        }
    }
    #[cfg(windows)]
    fn set_timeout(&self, sec: i32){
        unsafe {
            setsockopt(self.as_raw_socket(), SOL_SOCKET, SO_RCVTIMEO, &timeval{tv_sec: sec, tv_usec: 0} as *const _ as *const c_void, std::mem::size_of::<timeval>() as i32);
        }
    }
}
