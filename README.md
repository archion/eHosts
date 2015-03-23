# eHosts

eHosts is an enhanced hosts file that supports regex pattern match domain name. It is acturally a dns proxy run on udp 53 port.

Note: it is at very early stage and is written just for fun in rust-lang.

It is build on rust 1.0 nightly version and tested on GNU/Linux.



## Usage
First adding regex rules in hosts file start with `#$`, for example, if you want to match all domain name that contain `.google.com` to ip `192.168.0.1`, you may add in hosts file like:

```
#$ 192.168.0.1 .*\.google\.com
```
if you want to access google service in China, you may add below lines in your hosts file(note to replace `x.x.x.x` with a valid google ip)
```
#$ x.x.x.x .*google.*\.com.*
#$ x.x.x.x .*gstatic\.com
#$ x.x.x.x .*ggpht\.com
#$ x.x.x.x .*youtube.*\.com.*
#$ x.x.x.x .*ytimg\.com
```
Then build and run eHosts from Source:
```
$ git clone https://github.com/archion/eHosts
$ cd eHosts 
$ sudo Cargo run
```
and changing dns server setting to `127.0.0.1`.
