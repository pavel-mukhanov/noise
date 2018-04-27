extern crate byteorder;
extern crate bytes;
extern crate clap;
extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate snow;
extern crate tokio;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_service;

use clap::App;
use futures::future::Future;
use futures::Stream;
use std::net::SocketAddr;
use tokio_core::{reactor::Core, net::TcpListener};

mod noise_codec;
mod noise;

fn main() {
    let matches = App::new("simple")
        .args_from_usage("-s --server 'Server mode'")
        .get_matches();

    if matches.is_present("server") {
        run_server();
    } else {
        let socket_addr = "127.0.0.1:9999".parse().unwrap();
        send_message("", &socket_addr);
    }
    println!("all done.");
}

fn run_server() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let fut_stream = TcpListener::bind(&"127.0.0.1:9999".parse().unwrap(), &handle).unwrap();
    let fut = fut_stream.incoming()
        .map_err(|e| println!("failed to accept socket; error = {:?}", e))
        .for_each(|sock| {
            let reader = noise::noise_reader(sock.0);
            handle.spawn(reader);

            Ok(())
        });

    core.run(fut).expect("Running future!");
    println!("connection closed.");
}

fn send_message(_message: &str, addr: &SocketAddr) {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let stream = tokio_core::net::TcpStream::connect(&addr, &handle)
        .and_then(|sock| {
            noise::noise_writer(sock)
        });

    core.run(stream).unwrap();
}

