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
use futures::Sink;
use futures::Stream;
use snow::NoiseBuilder;
use snow::params::NoiseParams;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::error::Error as StdError;
use std::thread;
use tokio::prelude::future::ok;
use tokio_core::{reactor::Core, net::{TcpStream, TcpListener}};
use tokio_io::{AsyncRead, codec::LinesCodec};
use std::time::SystemTime;
use noise_codec::MessageCodec;
use tokio_service::{Service, NewService};
use tokio::io::WriteAll;
use tokio::io::ReadExact;
use futures::stream::AndThen;
use tokio::executor::current_thread;
use tokio_io::codec::Framed;

mod noise_codec;
mod noise;

static SECRET: &'static [u8] = b"i don't care for fidget spinners";
lazy_static! {
    static ref PARAMS: NoiseParams = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
}

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
    let mut buf = vec![0u8; 65535];

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let remote = core.remote();

    let fut_stream = TcpListener::bind(&"127.0.0.1:9999".parse().unwrap(), &handle).unwrap();
    let fut = fut_stream.incoming()
        .map_err(|e| println!("failed to accept socket; error = {:?}", e))
        .for_each(|sock| {
            let reader = noise::noise_reader(sock.0);
            handle.spawn(reader);

            Ok(())
        });

    core.run(fut);
    println!("connection closed.");
}

fn send_message(message: &str, addr: &SocketAddr) {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let mut stream = tokio_core::net::TcpStream::connect(&addr, &handle)
        .and_then(|sock| {
            noise::noise_writer(sock)
        });

    core.run(stream).unwrap();
}

