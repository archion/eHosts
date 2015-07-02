# eHosts [![Build Status](https://travis-ci.org/archion/eHosts.svg)](https://travis-ci.org/archion/eHosts)

eHosts is an enhanced hosts file that supports regex domain name matching (it is actually a dns proxy run on udp 53 port, so the priority is lower than the rules in system's hosts file).

Note: it is at very early stage and is written just for fun in rust-lang.

It is built on rust nightly and tested on GNU/Linux.


## Download binary file

If you are on platform list below, then you can download binary file directly (Anyone help building or maintaining the binary will be appreciated)

- [x86_64-linux](https://raw.githubusercontent.com/archion/eHosts/master/target/x86_64-unknown-linux-gnu/release/eHosts)
- [i686-linux](https://raw.githubusercontent.com/archion/eHosts/master/target/i686-unknown-linux-gnu/release/eHosts)
- [x86_64-windows](https://raw.githubusercontent.com/archion/eHosts/master/target/x86_64-pc-windows-gnu/release/eHosts.exe)

## Building from Source

```
$ git clone https://github.com/archion/eHosts
$ cd eHosts 
$ cargo build --release
$ sudo ./target/release/eHosts
```

## Usage
First adding regex rules in hosts file (default it use file named `hosts` in current working directory, you can use `-f` option to change it) start with `#$`, for example, if you want to match all domain name that contain `.google.com` to ip `192.168.0.1`, you may add in hosts file like:

```
#$ 192.168.0.1 .*\.google\.com
```
if you want to access google service in China, you may add below lines in your hosts file(note to replace `x.x.x.x` with a valid google ip, if you don't know how to find a valid google ip, see [here](http://archion.github.io/2014/06/18/%E8%87%AA%E5%AF%BB%E8%B0%B7%E6%AD%8C%E6%8C%A8%E5%B1%81/))
```
#$ x.x.x.x .*google.*\.com.*
#$ x.x.x.x .*gstatic\.com
#$ x.x.x.x .*ggpht\.com
#$ x.x.x.x .*youtube.*\.com.*
#$ x.x.x.x .*ytimg\.com
```
and changing dns server setting to `127.0.0.1` (on Linux, eHosts will set the dns for you by adding `nameserver 127.0.0.1` in /etc/resolv.conf).

```
eHosts -h

USAGE:
        eHosts [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -s               run in server mode
    -V, --version    Prints version information

OPTIONS:
    -d <addr>...         Set upstream DNS server [default: 8.8.8.8:53]
    -f <file>            Specify rule file, [default: ./hosts]
```



## To do list

- [x] instant update host rules
- [x] windows support
- [x] support multi dns and non 53 port for upstream dns via `-d` option
- [ ] ipv6 support
- [ ] dns cache
- [ ] improve dns lib
- [ ] tcp support
