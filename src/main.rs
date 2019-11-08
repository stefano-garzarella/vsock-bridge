use clap::{Arg, App, value_t, values_t, crate_authors, crate_version};
use vsock::{Vsock, VsockCid};
use nix::sys::{socket, epoll, /*sendfile::sendfile*/};
use std::{thread};

extern crate bytefmt;

const DEFAULT_BUF_SIZE: &str = "128KiB";

const EVENT_GUEST1_IN: u64 = 1;
const EVENT_GUEST2_IN: u64 = 2;

/* TODO: vsock doesn't support splice_read
fn bridge_sendfile(receiver: &Vsock, sender: &Vsock, buf_len: usize) ->
        Result<usize, nix::Error>
{
    sendfile(receiver.raw_fd(), sender.raw_fd(), None, buf_len)
}
*/

fn bridge_send(receiver: &Vsock, sender: &Vsock, buf_len: usize) ->
        Result<usize, nix::Error>
{
    //let mut buf = vec![0u8; buf_len];
    let mut buf: Vec<u8> = Vec::with_capacity(buf_len);
    unsafe {buf.set_len(buf_len)};

    let recv_len = match receiver.recv(&mut buf, socket::MsgFlags::empty()) {
        Ok(len) => len,
        Err(err) => return Err(err)
    };

    let mut off: usize = 0;

    while off < recv_len {
        let send_len = match sender.send(&buf[off .. recv_len],
                                         socket::MsgFlags::empty()) {
            Ok(len) => len,
            Err(err) => return Err(err)
        };

        off += send_len;
    }

    return Ok(off);
}

fn bridge(guest1: &Vsock, guest2: &Vsock, buf_len: usize)
{
    let mut event;
    let mut running = true;

    println!("Bridge thread started - guests(CID, port) g1: {:?} g2: {:?}",
             guest1.getpeername().unwrap(),
             guest2.getpeername().unwrap());

    let epoll_fd = epoll::epoll_create().unwrap();

    event = epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, EVENT_GUEST1_IN);
    epoll::epoll_ctl(epoll_fd, epoll::EpollOp::EpollCtlAdd, guest1.raw_fd(),
                     &mut event).unwrap();

    event = epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, EVENT_GUEST2_IN);
    epoll::epoll_ctl(epoll_fd, epoll::EpollOp::EpollCtlAdd, guest2.raw_fd(),
                     &mut event).unwrap();


    while running {
        let mut events = vec![epoll::EpollEvent::empty(); 10];

        let nfds = match epoll::epoll_wait(epoll_fd, &mut events, -1) {
            Ok(events) => events,
            Err(_) => break,
        };

        for event in events.iter().take(nfds) {
            let sender: &Vsock;
            let receiver: &Vsock;
            match event.data() {
                EVENT_GUEST1_IN => {
                    receiver = guest1;
                    sender = guest2;
                }
                EVENT_GUEST2_IN => {
                    receiver = guest2;
                    sender = guest1;
                }
                _ => {
                    panic!("Unknown event!");
                }
            }

            match bridge_send(receiver, sender, buf_len) {
                Ok(len) => {
                    if len == 0 {
                        running = false;
                        break;
                    }
                }
                Err(err) => {
                    eprintln!("{}", err);
                    running = false;
                    break;
                },
            };
        }
    }

    println!("Bridge thread ended - guests(CID, port) g1: {:?} g2: {:?}",
             guest1.getpeername().unwrap(),
             guest2.getpeername().unwrap());
}

fn main()
{
    let cmd_args = App::new("vsock-bridge")
        .version(crate_version!())
        .author(crate_authors!())
        .about("VSOCK bridge: creates a bridge between two VSOCK peers \
                on a specified port")
        .arg(
            Arg::with_name("guest")
                .long("guest")
                .short("g")
                .takes_value(true)
                .number_of_values(2)
                .required(true)
                .help("Guest CIDs to bridge"),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .short("p")
                .takes_value(true)
                .required(true)
                .help("Port number to bridge"),
        )
        .arg(
            Arg::with_name("length")
                .long("length")
                .short("l")
                .takes_value(true)
                .help("buffer length used to move data between sockets \
                       [def. 128KiB]"),
        )
        .get_matches();

    let port = value_t!(cmd_args, "port", u32).unwrap();
    let guests = values_t!(cmd_args, "guest", u32).unwrap();
    let length = cmd_args.value_of("length").unwrap_or(DEFAULT_BUF_SIZE);

    let buf_len = bytefmt::parse(length).unwrap() as usize;

    println!("Bridge starting CIDs: {} <-> {} port: {} buf_len: {}",
             guests[0], guests[1], port, buf_len);

    let vsock = Vsock::new();

    vsock.bind(VsockCid::any(), port).unwrap();
    vsock.listen(10).expect("Unable to listen");

    loop {
        println!("Listening on port {} ...", port);
        let guest1_vsock = match vsock.accept() {
            Ok(vsock) => vsock,
            Err(err) => {
                eprintln!("Unable to accept {}", err);
                continue;
            }
        };
        let (guest1_cid, _) = guest1_vsock.getpeername().unwrap();
        let guest2_cid: u32;

        if guest1_cid == guests[0] {
            guest2_cid = guests[1];
        } else if guest1_cid == guests[1] {
            guest2_cid = guests[0];
        } else {
            eprintln!("Unexpected guest [cid: {}]", guest1_cid);
            continue;
        }

        let guest2_vsock = Vsock::new();
        if let Err(err) = guest2_vsock.connect(guest2_cid, port) {
            eprintln!("Unable to connect {}", err);
            continue;
        }

        let _handle = thread::spawn(move || {
            bridge(&guest1_vsock, &guest2_vsock, buf_len);
        });
    }
}
