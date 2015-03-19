# eHosts

eHosts is an enhanced hosts file that supports regex pattern match domain name. It is acturally a dns proxy run on udp 53 port.

Note: it is at very early stage and is written just for fun in rust-lang.

It is build on rust 1.0 nightly version

## Usage
First adding regex rules in hosts file start with `#$`, for example, if you want to match all domain name that contain `.google.com`, you may add in hosts file like:

```
#$ 192.168.0.0 .*\.google\.com
```
then changing dns server setting to `127.0.0.1`.
