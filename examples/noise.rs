//! This is a barebones TCP Client/Server that establishes a `Noise_NN` session, and sends
//! an important message across the wire.
//!
//! # Usage
//! Run the server a-like-a-so `cargo run --example simple -- -s`, then run the client
//! as `cargo run --example simple` to see the magic happen.

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


use clap::App;
use codec::MessageCodec;
use futures::future::{self, Future};
use futures::future::ok;
use futures::Sink;
use futures::Stream;
use snow::NoiseBuilder;
use snow::params::NoiseParams;
use snow::Session;
use std::error::Error as StdError;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::SystemTime;
use tokio::prelude::*;
use tokio_core::net;
use tokio_core::reactor::Core;
use tokio_io::AsyncRead;
use tokio_io::codec::BytesCodec;

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
        run_client();
    }
    println!("all done.");
}

fn run_server() {
    let mut buf = vec![0u8; 65535];

    // Initialize our responder NoiseSession using a builder.
    let builder: NoiseBuilder = NoiseBuilder::new(PARAMS.clone());
    let static_key = builder.generate_private_key().unwrap();
    let mut noise: Session = builder
        .local_private_key(&static_key)
        .psk(3, SECRET)
        .build_responder().unwrap();

    // Wait on our client's arrival...
    println!("listening on 127.0.0.1:9999");
    {
        let listener = TcpListener::bind("127.0.0.1:9999").unwrap();
        {
            let (mut stream, _) = listener.accept().unwrap();

            // <- e
            noise.read_message(&recv(&mut stream).unwrap(), &mut buf).unwrap();

            // -> e, ee, s, es
            let len = noise.write_message(&[0u8; 0], &mut buf).unwrap();
            send(&mut stream, &buf[..len]);

            // <- s, se
            noise.read_message(&recv(&mut stream).unwrap(), &mut buf).unwrap();

            // Transition the state machine into transport mode now that the handshake is complete.


//    let fut = future::result(fut_stream);
        }

//    while let Ok(msg) = recv(&mut stream) {
//        let len = noise.read_message(&msg, &mut buf).unwrap();
//        println!("client said: {}", String::from_utf8_lossy(&buf[..len]));
//    }

        let mut core = Core::new().unwrap();
        let handle = core.handle();


        let fut_stream = tokio_core::net::TcpListener::from_listener(listener, &"127.0.0.1:9999".parse().unwrap(), &handle).unwrap();
        let fut = fut_stream.incoming()
            .map_err(|e| println!("failed to accept socket; error = {:?}", e))
            .for_each(move |s| {
//            noise_message(&mut noise);

                let mut noise: Session = noise.into_transport_mode().unwrap();
                let (sink, stream) =
                    s.0.framed(MessageCodec::new(noise)).split();
//
//                let connection_handler = stream
//                    .into_future()
//                    .map_err(|e| e.0)
//                    .and_then(move |(raw, stream)| {
//                        println!("raw {:?}", raw);
//                        Ok((raw, stream))
//                    })
//                    .map_err(log_error)
//                    .and_then(|x| {
//                        ok(())
//                    });
//
//
//            handle.spawn(connection_handler);

                ok(())
            });


        core.run(fut).unwrap();
    }

    println!("connection closed.");
}

pub fn log_error<E: StdError>(err: E) {
    println!("An error occurred: {}", err)
}

fn respond(req: Vec<u8>)
           -> Box<Future<Item=(), Error=io::Error>>
{
    Box::new(future::ok(()))
}

fn run_client() {
    let mut buf = vec![0u8; 65535];

    // Initialize our initiator NoiseSession using a builder.
    let builder: NoiseBuilder = NoiseBuilder::new(PARAMS.clone());
    let static_key = builder.generate_private_key().unwrap();
    let mut noise = builder
        .local_private_key(&static_key)
        .psk(3, SECRET)
        .build_initiator()
        .unwrap();

    // Connect to our server, which is hopefully listening.
    let mut stream = TcpStream::connect("127.0.0.1:9999").unwrap();
    println!("connected...");

    // -> e
    let len = noise.write_message(&[], &mut buf).unwrap();
    send(&mut stream, &buf[..len]);

    // <- e, ee, s, es
    noise
        .read_message(&recv(&mut stream).unwrap(), &mut buf)
        .unwrap();

    // -> s, se
    let len = noise.write_message(&[], &mut buf).unwrap();
    send(&mut stream, &buf[..len]);

    let mut noise = noise.into_transport_mode().unwrap();
    println!("session established...");

    // Get to the important business of sending secured data.
    for _ in 0..10 {
        let len = noise.write_message(b"HACK THE PLANET", &mut buf).unwrap();
        send(&mut stream, &buf[..len]);
    }

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let fut_steam = tokio_core::net::TcpStream::from_stream(stream, &handle);
    let fut = future::result(fut_steam).and_then(|x| {
        println!("stream {:?}", x);
        ok(())
    });

    core.run(fut).unwrap();

    println!("notified server of intent to hack planet.");
}

/// Hyper-basic stream transport receiver. 16-bit BE size followed by payload.
fn recv(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut msg_len_buf = [0u8; 2];
    println!("read exact...");
    stream.read_exact(&mut msg_len_buf)?;
    println!("read exact second..., msg_len_buf {:?}", msg_len_buf);
    let msg_len = ((msg_len_buf[0] as usize) << 8) + (msg_len_buf[1] as usize);
    let mut msg = vec![0u8; msg_len];
    stream.read_exact(&mut msg[..])?;

    println!("read exact second..., msg {:?}", msg);
    Ok(msg)
}

/// Hyper-basic stream transport sender. 16-bit BE size followed by payload.
fn send(stream: &mut TcpStream, buf: &[u8]) {
    let msg_len_buf = [(buf.len() >> 8) as u8, (buf.len() & 0xff) as u8];
    println!("msg_len_buf {:?}", msg_len_buf);
    stream.write_all(&msg_len_buf).unwrap();

    let buf2 = buf.clone();
    stream.write_all(buf).unwrap();

    let len = msg_len_buf[1] as usize;
    println!("buf len {}, buf {:?}", len, &buf2[0..len]);
}
