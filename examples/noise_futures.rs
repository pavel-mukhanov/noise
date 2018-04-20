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
use codec::MessageCodec;
use tokio_service::{Service, NewService};
use tokio::io::WriteAll;
use tokio::io::ReadExact;
use futures::stream::AndThen;

mod codec;

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
            let builder: NoiseBuilder = NoiseBuilder::new(PARAMS.clone());
            let static_key = builder.generate_private_key().unwrap();
            let mut noise = builder
                .local_private_key(&static_key)
                .psk(3, SECRET)
                .build_responder()
                .unwrap();

            let reader =
                read(sock.0)
                    .and_then(move |s| {
                        let mut buf = vec![0u8; 65535];

                        // <- e
                        noise
                            .read_message(&s.1, &mut buf);

                        // -> e, ee, s, es
                        let len = noise.write_message(&[0u8; 0], &mut buf).unwrap();

                        write(s.0, buf, len)
                            .and_then(|s| {
                                read(s.0)
                            })
                            .and_then(move |s| {
                                let readed_buf = s.1;
                                let mut buf = vec![0u8; 65535];
                                // <- s, se
                                noise.read_message(&readed_buf, &mut buf)
                                    .unwrap();

                                let mut noise = noise.into_transport_mode().unwrap();

                                let (sink, stream) =
                                    s.0.framed(MessageCodec::new(noise)).split();

                                stream
                                    .into_future()
                                    .map_err(|e| e.0)
                                    .and_then(|s| {
                                        println!("client said: {:?}", s.0);
                                        Ok((s.0, s.1))
                                    })
                                    .map_err(log_error)
                                    .then(|s| {
                                        Ok(())
                                    })
                            })
                    })
                    .then(|x| {
                        Ok(())
                    });
            handle.spawn(to_box(reader));

            Ok(())
        });

    core.run(fut);
    println!("connection closed.");
}

fn noise_reader() {


}


fn to_box<F: Future + 'static>(f: F) -> Box<Future<Item=(), Error=F::Error>> {
    Box::new(f.map(drop))
}

fn send_message(message: &str, addr: &SocketAddr) {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let mut stream = tokio_core::net::TcpStream::connect(&addr, &handle)
        .and_then(|sock| {
            let builder: NoiseBuilder = NoiseBuilder::new(PARAMS.clone());
            let static_key = builder.generate_private_key().unwrap();
            let mut noise = builder
                .local_private_key(&static_key)
                .psk(3, SECRET)
                .build_initiator()
                .unwrap();

            println!("connected to {:?}, time {:?}", sock, SystemTime::now());
            let mut buf = vec![0u8; 65535];
            // -> e
            let len = noise.write_message(&[], &mut buf).unwrap();
            write(sock, buf, len)
                .and_then(|sock| {
                    read(sock.0)
                })
                .and_then(|sock| {
                    let readed_buf = sock.1;
                    println!("readed buf {:?}", readed_buf);
                    let mut buf = vec![0u8; 65535];
                    // <- e, ee, s, es
                    noise
                        .read_message(&readed_buf, &mut buf)
                        .unwrap();

                    let len = noise.write_message(&[], &mut buf).unwrap();
                    let buf = &buf[0..len];
                    write(sock.0, Vec::from(buf), len)
                        .and_then(|sock| {
                            let mut noise = noise.into_transport_mode().unwrap();
                            let (sink, stream) =
                                sock.0.framed(MessageCodec::new(noise)).split();
                            sink.send(String::from("REALLY IMPORTANT ENCRYPTED MESSAGE"))
                        })
                })
                .then(|x| {
                    ok(())
                })
        })
        .map_err(|e| eprintln!("Error: {}", e));

    core.run(stream).unwrap();
}

pub fn read(sock: tokio_core::net::TcpStream) -> Box<Future<Item=(TcpStream, Vec<u8>), Error=io::Error>> {
    let mut buf = vec![0u8; 2];
    Box::new(tokio::io::read_exact(sock, buf).and_then(|sock| {
        tokio::io::read_exact(sock.0, vec![0u8; sock.1[1] as usize])
    }))
}

pub fn write(sock: TcpStream, buf: Vec<u8>, len: usize) -> Box<Future<Item=(TcpStream, Vec<u8>), Error=io::Error>> {
    let mut msg_len_buf = vec![(len >> 8) as u8, (len & 0xff) as u8];
    let buf = &buf[0..len];
    msg_len_buf.extend_from_slice(buf);
    println!("write {:?}", msg_len_buf);
    Box::new(tokio::io::write_all(sock, msg_len_buf))
}

pub fn log_error<E: StdError>(err: E) {
    println!("An error occurred: {}", err)
}
