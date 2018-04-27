extern crate byteorder;
extern crate bytes;
extern crate clap;
extern crate futures;
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

//static SECRET: &'static [u8] = b"i don't care for fidget spinners";
static SECRET: &'static [u8] = b"secret secret secret key secrets";
lazy_static! {
    static ref PARAMS: NoiseParams = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
}

pub fn handshake_listen(stream: TcpStream) -> Box<Future<Item=Framed<TcpStream, MessageCodec>, Error=io::Error>> {
    let builder: NoiseBuilder = NoiseBuilder::new(PARAMS.clone());
    let static_key = builder.generate_private_key().unwrap();
    let mut noise = builder
        .local_private_key(&static_key)
        .psk(3, SECRET)
        .build_responder()
        .unwrap();

    let framed = read(stream)
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
                    let framed = s.0.framed(MessageCodec::new(noise));
                    Ok(framed)
                })
        });

    Box::new(framed)
}

pub fn handshake_sender(stream: TcpStream) -> Box<Future<Item=Framed<TcpStream, MessageCodec>, Error=io::Error>> {
    let builder: NoiseBuilder = NoiseBuilder::new(PARAMS.clone());
    let static_key = builder.generate_private_key().unwrap();
    let mut noise = builder
        .local_private_key(&static_key)
        .psk(3, SECRET)
        .build_initiator()
        .unwrap();

    println!("connected to {:?}, time {:?}", stream, SystemTime::now());
    let mut buf = vec![0u8; 65535];
    // -> e
    let len = noise.write_message(&[], &mut buf).unwrap();
    let framed
    = write(stream, buf, len)
        .and_then(|sock| {
            read(sock.0)
        })
        .and_then(|sock| {
            let readed_buf = sock.1;
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
                    let framed = sock.0.framed(MessageCodec::new(noise));
                    Ok(framed)
                })
        });

    Box::new(framed)
}

pub fn noise_reader(stream: TcpStream) -> Box<Future<Item=(), Error=()>> {
    let builder: NoiseBuilder = NoiseBuilder::new(PARAMS.clone());
    let static_key = builder.generate_private_key().unwrap();
    let mut noise = builder
        .local_private_key(&static_key)
        .psk(3, SECRET)
        .build_responder()
        .unwrap();

    let reader =
        handshake_listen(stream)
            .and_then(|framed| {
                let (sink, stream) = framed.split();

                let conn = stream.for_each(|msg| {
                    println!("Got a message from client {:?}", msg);
                    Ok(())
                })
                    .map_err(log_error)
                    .then(|s| {
                        Ok(())
                    });
                current_thread::spawn(conn);
                Ok(())
            })
            .then(|x| {
                Ok(())
            });

    Box::new(reader)
}

pub fn noise_writer(stream: TcpStream) -> Box<Future<Item=(), Error=io::Error>> {
    let writer =
        handshake_sender(stream).and_then(|framed| {
            let (sink, stream) = framed.split();
            sink.send(String::from("REALLY IMPORTANT ENCRYPTED MESSAGE"))
                .and_then(|sink| {
                    sink.send(String::from("SECOND REALLY IMPORTANT ENCRYPTED MESSAGE"))
                })
                .and_then(|sink| {
                    sink.send(String::from("THIRD REALLY IMPORTANT ENCRYPTED MESSAGE"))
                })
        })
            .then(|x| {
                Ok(())
            });
    Box::new(writer)
}

fn to_box<F: Future + 'static>(f: F) -> Box<Future<Item=(), Error=F::Error>> {
    Box::new(f.map(drop))
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
    Box::new(tokio::io::write_all(sock, msg_len_buf))
}

pub fn log_error<E: StdError>(err: E) {
    println!("An error occurred: {}", err)
}
