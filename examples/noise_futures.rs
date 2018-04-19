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
use futures::future::Future;
use futures::Sink;
use futures::Stream;
use snow::NoiseBuilder;
use snow::params::NoiseParams;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use std::thread;
use tokio::prelude::future::ok;
use tokio_core::reactor::Core;
use tokio_io::{AsyncRead, codec::LinesCodec};
use std::time::SystemTime;

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

    // Initialize our responder NoiseSession using a builder.
    let builder: NoiseBuilder = NoiseBuilder::new(PARAMS.clone());
    let static_key = builder.generate_private_key().unwrap();
    let mut noise = builder
        .local_private_key(&static_key)
        .psk(3, SECRET)
        .build_responder()
        .unwrap();

    // Wait on our client's arrival...
    println!("listening on 127.0.0.1:9999");
    let (mut stream, _) = TcpListener::bind("127.0.0.1:9999")
        .unwrap()
        .accept()
        .unwrap();

    // <- e
    noise
        .read_message(&recv(&mut stream).unwrap(), &mut buf)
        .unwrap();

    // -> e, ee, s, es
    let len = noise.write_message(&[0u8; 0], &mut buf).unwrap();
    send(&mut stream, &buf[..len]);

    // <- s, se
    noise
        .read_message(&recv(&mut stream).unwrap(), &mut buf)
        .unwrap();

    // Transition the state machine into transport mode now that the handshake is complete.
    let mut noise = noise.into_transport_mode().unwrap();

    while let Ok(msg) = recv(&mut stream) {
        let len = noise.read_message(&msg, &mut buf).unwrap();
        println!("client said: {}", String::from_utf8_lossy(&buf[..len]));
    }
    println!("connection closed.");
}

fn send_message(message: &str, addr: &SocketAddr) {
    let mut core = Core::new().unwrap();
    let handle = core.handle();


    // Initialize our initiator NoiseSession using a builder.
    let builder: NoiseBuilder = NoiseBuilder::new(PARAMS.clone());
    let static_key = builder.generate_private_key().unwrap();
    let mut noise = builder
        .local_private_key(&static_key)
        .psk(3, SECRET)
        .build_initiator()
        .unwrap();

    let mut stream = tokio_core::net::TcpStream::connect(&addr, &handle)
        .and_then(|sock| {
            println!("connected to {:?}, time {:?}", sock, SystemTime::now());
            let mut buf = vec![0u8; 65535];

            // -> e
            let len = noise.write_message(&[], &mut buf).unwrap();
            let mut msg_len_buf = vec![(len >> 8) as u8, (len & 0xff) as u8];
            msg_len_buf.extend_from_slice(buf.as_slice());
            tokio::io::write_all(sock, msg_len_buf)
                .and_then(|sock| {
                    let socket = sock.0;
                    println!("second read, buf len {:?}", sock.1[1]);
                    let buf_len = sock.1[1] as usize;
                    let mut size_buf = vec![0, 0];
                    let mut buf = vec![0u8; 98];

                    println!("read_to_end, buf len {:?}", buf.len());
                    tokio::io::read_exact(socket, buf)
                })
                .and_then(|sock| {
                    let len = sock.1[1] as usize;
                    println!("second NOISE read, buf len {:?}, buf {:?}", len, sock.1);
                    let mut buf = vec![0u8; 65535];

                    let mut buf_to_read = &sock.1[2..len + 2];
                    println!("buf to read {:?}", buf_to_read);
                    // <- e, ee, s, es
                    noise
                        .read_message(&buf_to_read, &mut buf)
                        .unwrap();

                    println!("second NOISE write, time {:?}", SystemTime::now());
                    let mut buf = vec![0u8; 65535];
                    let len = noise.write_message(&[], &mut buf).unwrap();

                    let mut msg_len_buf = vec![(len >> 8) as u8, (len & 0xff) as u8];
                    let buf = &buf[0..len];
                    msg_len_buf.extend_from_slice(buf);
                    println!("second NOISE writte buf {:?},", buf);
                    tokio::io::write_all(sock.0, msg_len_buf)
                        .and_then(|sock| {
                            println!("second NOISE written buf {:?}, time {:?}", sock.1, SystemTime::now());
                            let mut noise = noise.into_transport_mode().unwrap();

                            let mut buf = vec![0u8; 65535];
                            let len = noise.write_message(b"HACK THE PLANET", &mut buf).unwrap();

                            let mut msg_len_buf = vec![(len >> 8) as u8, (len & 0xff) as u8];
                            let buf = &buf[0..len];
                            msg_len_buf.extend_from_slice(buf);
//                            tokio::io::write_all(sock.0, msg_len_buf).and_then(|sock| {
//                                println!("transport message!");
//                                ok(())
//                            })
                            ok(())
                        })
                })
                .then(|x| {
                    ok(())
                })
        })
        .map_err(|e| eprintln!("Error: {}", e));

    core.run(stream).unwrap();

    thread::sleep_ms(1000);
}

/// Hyper-basic stream transport receiver. 16-bit BE size followed by payload.
fn recv(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut msg_len_buf = [0u8; 2];
    stream.read_exact(&mut msg_len_buf)?;

    let msg_len = ((msg_len_buf[0] as usize) << 8) + (msg_len_buf[1] as usize);
    let mut msg = vec![0u8; msg_len];
    stream.read_exact(&mut msg[..])?;
    Ok(msg)
}

/// Hyper-basic stream transport sender. 16-bit BE size followed by payload.
fn send(stream: &mut TcpStream, buf: &[u8]) {
    let msg_len_buf = [(buf.len() >> 8) as u8, (buf.len() & 0xff) as u8];
    stream.write_all(&msg_len_buf).unwrap();
    stream.write_all(buf).unwrap();
}