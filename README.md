# vsock-bridge

`vsock-bridge` is an user space application that allow you to create a bridge
between VSOCK peers on a specified port, allowing VM to VM, or L2 to L0
communication.

## Build

``` shell
git clone https://github.com/stefano-garzarella/vsock-bridge

cd vsock-bridge

# debug version: ./target/debug/vsock-bridge
cargo build

# release version: ./target/release/vsock-bridge
cargo build --release
```

## Install

These steps install `vsock-bridge` [release version] in the default
installation root (e.g. `$HOME/.cargo`)

``` shell
cd vsock-bridge

cargo install --path .
```

## Usage

``` shell
$ vsock-bridge -h
vsock-bridge 0.1.0
Stefano Garzarella <sgarzare@redhat.com>
VSOCK bridge: creates a bridge between two VSOCK peers on a specified port

USAGE:
    vsock-bridge [OPTIONS] --guest <guest> <guest> --port <port>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -g, --guest <guest> <guest>    Guest CIDs to bridge
    -l, --length <length>          buffer length used to move data between sockets [def. 128KiB]
    -p, --port <port>              Port number to bridge
```

## Example

1. Create a bridge between sibling guests [CIDs 3 and 4] and run
   [iperf-vsock](https://github.com/stefano-garzarella/iperf-vsock) on the
   default port [5201] between them:
   - host
     ``` shell
     $ vsock-bridge -g 3 4 --port 5201
     Bridge starting CIDs: 3 <-> 4 port: 5201 buf_len: 131072
     Listening on port 5201 ...
     Listening on port 5201 ...
     Bridge thread started - guests(CID, port) g1: (4, 545519714) g2: (3, 5201)
     Listening on port 5201 ...
     Bridge thread started - guests(CID, port) g1: (4, 545519715) g2: (3, 5201)
     ```
   - VM 1 [CID: 3]
     ``` shell
     $ iperf --vsock -s
     -----------------------------------------------------------
     Server listening on 5201
     -----------------------------------------------------------
     Accepted connection from 2, port 2032126466
     [  5] local 3 port 5201 connected to 2 port 2032126467
     [ ID] Interval           Transfer     Bitrate
     [  5]   0.00-1.00   sec   439 MBytes  3.68 Gbits/sec
     [  5]   1.00-2.00   sec   496 MBytes  4.16 Gbits/sec
     ```
   - VM 2 [CID: 4]
     ``` shell
     $ iperf --vsock -c 2
     Connecting to host 2, port 5201
     [  5] local 4 port 545519715 connected to 2 port 5201
     [ ID] Interval           Transfer     Bitrate
     [  5]   0.00-1.00   sec   440 MBytes  3.69 Gbits/sec
     [  5]   1.00-2.00   sec   496 MBytes  4.16 Gbits/sec
     ```

   **Note**: In the guests (VM 2 in this example), we must use the host CID to
   address the other VM, since the bridge is running in the host, and it
   represents the end point of communication.
