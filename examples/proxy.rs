//! MySQL Proxy Server

#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate futures;
#[macro_use]
extern crate tokio_core;
extern crate futures_cpupool;
extern crate mysql_proxy;

use mysql_proxy::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::env;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
use std::net::Shutdown;
use std::str;
use std::time::Duration;

use futures::{Future, Poll};
use futures::stream::Stream;
use futures_cpupool::CpuPool;
use tokio_core::{Loop, LoopHandle, TcpStream};
use tokio_core::io::{read_exact, write_all, Window};


fn main() {
    drop(env_logger::init());

    let addr = env::args().nth(1).unwrap_or("127.0.0.1:3307".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();

    let mut lp = Loop::new().unwrap();
    let pool = CpuPool::new(4);
    let buffer = Rc::new(RefCell::new(vec![0; 64 * 1024]));
    let handle = lp.handle();
    let listener = lp.run(handle.clone().tcp_listen(&addr)).unwrap();
    let pin = lp.pin();

    println!("Listening for MySQL proxy connections on {}", addr);
    let clients = listener.incoming().map(move |(socket, addr)| {
        (Client {
            pool: pool.clone(),
            handle: handle.clone(),
        }.serve(socket), addr)
    });
    let server = clients.for_each(|(client, addr)| {
        pin.spawn(client.then(move |res| {
            match res {
                Ok((a, b)) => {
                    println!("proxied {}/{} bytes for {}", a, b, addr)
                }
                Err(e) => println!("error for {}: {}", addr, e),
            }
            futures::finished(())
        }));
        Ok(())
    });

    lp.run(server).unwrap();
}
