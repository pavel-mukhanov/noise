// Copyright 2018 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use byteorder::{ByteOrder, LittleEndian};
use futures::future::{done, Future};
use noise_codec::MessagesCodec;
use std::io;
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, codec::Framed, io::{read_exact, write_all}};
use wrapper::HANDSHAKE_HEADER_LENGTH;
use wrapper::HandshakeParams;
use wrapper::NoiseWrapper;

pub type HandshakeResult = Box<Future<Item=Framed<TcpStream, MessagesCodec>, Error=io::Error>>;

#[derive(Debug)]
pub struct NoiseHandshake {}

impl NoiseHandshake {
    pub fn listen(params: &HandshakeParams, stream: TcpStream) -> HandshakeResult {
        listen_handshake(stream, params)
    }

    pub fn send(params: &HandshakeParams, stream: TcpStream) -> HandshakeResult {
        send_handshake(stream, params)
    }
}

fn listen_handshake(stream: TcpStream, params: &HandshakeParams) -> HandshakeResult {
    let max_message_len = params.max_message_len;
    let mut noise = NoiseWrapper::responder(params);
    let framed = read(stream).and_then(move |(stream, msg)| {
        read_handshake_msg(&msg, &mut noise)
            .and_then(|_| {
                write_handshake_msg(&mut noise)
                    .and_then(|(len, buf)| write(stream, &buf, len))
                    .and_then(|(stream, _msg)| read(stream))
                    .and_then(move |(stream, msg)| {
                        let _buf = noise.read_handshake_msg(&msg)?;
                        let noise = noise.into_transport_mode()?;
                        let framed = stream.framed(MessagesCodec::new(noise));
                        Ok(framed)
                    })
            })
    });

    Box::new(framed)
}

fn send_handshake(stream: TcpStream, params: &HandshakeParams) -> HandshakeResult {
    let max_message_len = params.max_message_len;
    let mut noise = NoiseWrapper::initiator(params);
    let framed = write_handshake_msg(&mut noise)
        .and_then(|(len, buf)| write(stream, &buf, len))
        .and_then(|(stream, _msg)| read(stream))
        .and_then(move |(stream, msg)| {
            read_handshake_msg(&msg, &mut noise)
                .and_then(|_| {
                    write_handshake_msg(&mut noise)
                        .and_then(|(len, buf)| write(stream, &buf, len))
                        .and_then(move |(stream, _msg)| {
                            let noise = noise.into_transport_mode()?;
                            let framed = stream.framed(MessagesCodec::new(noise));
                            Ok(framed)
                        })
                })
        });

    Box::new(framed)
}

pub fn read(sock: TcpStream) -> Box<Future<Item=(TcpStream, Vec<u8>), Error=io::Error>> {
    let buf = vec![0u8; HANDSHAKE_HEADER_LENGTH];
    Box::new(
        read_exact(sock, buf)
            .and_then(|(stream, msg)| read_exact(stream, vec![0u8; msg[0] as usize])),
    )
}

pub fn write(
    sock: TcpStream,
    buf: &[u8],
    len: usize,
) -> Box<Future<Item=(TcpStream, Vec<u8>), Error=io::Error>> {
    let mut message = vec![0u8; HANDSHAKE_HEADER_LENGTH];
    LittleEndian::write_u16(&mut message, len as u16);
    message.extend_from_slice(&buf[0..len]);
    Box::new(write_all(sock, message))
}

pub fn read_handshake_msg(
    input: &[u8],
    noise: &mut NoiseWrapper,
) -> Box<Future<Item=(usize, Vec<u8>), Error=io::Error>> {
    let res = noise.read_handshake_msg(input);
    Box::new(done(res.map_err(|e| e.into())))
}

pub fn write_handshake_msg(
    noise: &mut NoiseWrapper,
) -> Box<Future<Item=(usize, Vec<u8>), Error=io::Error>> {
    let res = noise.write_handshake_msg();
    Box::new(done(res.map_err(|e| e.into())))
}

mod tests {
    use byteorder::{ByteOrder, LittleEndian};
    use env_logger;
    use futures::{done, Future, Stream};
    use futures::unsync;
    use futures::unsync::oneshot::Receiver;
    use futures::unsync::oneshot::Sender;
    use noise_codec::MessagesCodec;
    use noise_main::HandshakeResult;
    use noise_main::NoiseHandshake;
    use noise_main::read;
    use noise_main::read_handshake_msg;
    use noise_main::write;
    use noise_main::write_handshake_msg;
    use snow::NoiseBuilder;
    use snow::params::NoiseParams;
    use snow::Session;
    use std::error::Error as StdError;
    use std::io::{Read, Write};
    use std::io;
    use std::net::{TcpListener as StdTcpListener, TcpStream as StdTcpStream};
    use std::net::{SocketAddr, ToSocketAddrs};
    use std::sync::Arc;
    use std::thread;
    use tokio_core::net::{TcpListener, TcpStream};
    use tokio_core::reactor::Core;
    use tokio_io::AsyncRead;
    use wrapper::HandshakeParams;
    use wrapper::NoiseError;
    use wrapper::NoiseWrapper;
    use wrapper::NOISE_MIN_HANDSHAKE_MESSAGE_LENGTH;
    use wrapper::NOISE_MAX_HANDSHAKE_MESSAGE_LENGTH;

    #[derive(Debug, PartialEq, Copy, Clone)]
    pub enum Step {
        Normal,
        One(u8, usize),
        Two(u8, usize),
    }

    const EMPTY_MESSAGE: usize = 0;
    const SMALL_MESSAGE: usize = NOISE_MIN_HANDSHAKE_MESSAGE_LENGTH - 1;
    const BIG_MESSAGE: usize = NOISE_MAX_HANDSHAKE_MESSAGE_LENGTH + 1;

    #[test]
    fn test_noise_normal_handshake() {
        let addr: SocketAddr = "127.0.0.1:5001".parse().unwrap();
        let addr2 = addr.clone();

        thread::spawn(move || {
            run_handshake_listener(&addr2, Step::Normal)
        });

        let res = send_handshake(&addr, Step::Normal);
        assert!(res.is_ok());
    }

    #[test]
    fn test_noise_bad_handshake() {
        let addr:SocketAddr  = "127.0.0.1:5002".parse().unwrap();
        let addr2 = addr.clone();

        thread::spawn(move || {
            run_handshake_listener(&addr2, Step::Normal)
        });

        let res = send_handshake(&addr, Step::One(1, EMPTY_MESSAGE));
        assert!(res.is_err());

        let res = send_handshake(&addr, Step::Two(2, EMPTY_MESSAGE));
        assert!(res.is_err());

        let res = send_handshake(&addr, Step::One(1, SMALL_MESSAGE));
        assert!(res.is_err());

        let res = send_handshake(&addr, Step::Two(2, SMALL_MESSAGE));
        assert!(res.is_err());

        let res = send_handshake(&addr, Step::One(1, BIG_MESSAGE));
        assert!(res.is_err());

        let res = send_handshake(&addr, Step::Two(2, BIG_MESSAGE));
        assert!(res.is_err());
    }

    #[test]
    fn test_noise_bad_listen() {
        env_logger::init();
        test_noise_bad_listener(&"127.0.0.1:5003".parse().unwrap(), EMPTY_MESSAGE);
        test_noise_bad_listener(&"127.0.0.1:5004".parse().unwrap(), SMALL_MESSAGE);
        test_noise_bad_listener(&"127.0.0.1:5005".parse().unwrap(), BIG_MESSAGE);
    }

    fn test_noise_bad_listener(addr: &SocketAddr, message_size: usize) {
        let addr2 = addr.clone();

        thread::spawn(move || {
            run_handshake_listener(&addr2, Step::One(1, message_size))
        });

        let res = send_handshake(&addr, Step::Normal);
        assert!(res.is_err());
    }

    fn run_handshake_listener(addr: &SocketAddr, step: Step) -> Result<(), io::Error> {
        let mut core = Core::new().unwrap();
        let handle = core.handle();

        let params = HandshakeParams {
            max_message_len: 1024,
        };

        let fut_stream = TcpListener::bind(addr, &handle).unwrap();
        let fut = fut_stream.incoming()
            .for_each(|(stream, _)| {
                let reader =
                    listen_bad_handshake(stream, &params, step)
                        .and_then(|framed| {
                            Ok(())
                        }).map_err(log_error);

                handle.spawn(reader);
                Ok(())
            })
            .map_err(into_other);

        core.run(fut)
    }

    fn send_handshake(addr: &SocketAddr, step: Step) -> Result<(), io::Error> {
        let mut core = Core::new().unwrap();
        let handle = core.handle();

        let params = HandshakeParams {
            max_message_len: 1024,
        };

        let stream = TcpStream::connect(&addr, &handle)
            .and_then(|sock| {
                send_bad_handshake(&params, sock, step)
            })
            .map(|_| {
                ()
            })
            .map_err(into_other);

        core.run(stream)
    }

    pub fn log_error<E: StdError>(err: E) {
        error!("An error occurred: {}", err)
    }

    pub fn other_error<S: AsRef<str>>(s: S) -> io::Error {
        io::Error::new(io::ErrorKind::Other, s.as_ref())
    }

    pub fn into_other<E: StdError>(err: E) -> io::Error {
        other_error(&format!("An error occurred, {}", err.description()))
    }

    fn send_bad_handshake(params: &HandshakeParams, stream: TcpStream, step: Step) -> HandshakeResult {
        let max_message_len = params.max_message_len;
        let mut noise = NoiseWrapper::initiator(params);
        let framed
        = write_bad_handshake_msg(&mut noise, 1, &step)
            .and_then(|(len, buf)| write(stream, &buf, len))
            .and_then(|(stream, _msg)| read(stream))
            .and_then(move |(stream, msg)| {
                read_handshake_msg(&msg, &mut noise)
                    .and_then(move |_| {
                        write_bad_handshake_msg(&mut noise, 2, &step)
                            .and_then(|(len, buf)| write(stream, &buf, len))
                            .and_then(move |(stream, _msg)| {
                                let noise = noise.into_transport_mode()?;
                                let framed = stream.framed(MessagesCodec::new(noise));
                                Ok(framed)
                            })
                    })
            });

        Box::new(framed)
    }

    fn listen_bad_handshake(stream: TcpStream, params: &HandshakeParams, step: Step) -> HandshakeResult {
        let max_message_len = params.max_message_len;
        let mut noise = NoiseWrapper::responder(params);
        let framed = read(stream).and_then(move |(stream, msg)| {
            read_handshake_msg(&msg, &mut noise)
                .and_then(move |_| {
                    write_bad_handshake_msg(&mut noise, 1, &step)
                        .and_then(|(len, buf)| write(stream, &buf, len))
                        .and_then(|(stream, _msg)| read(stream))
                        .and_then(move |(stream, msg)| {
                            noise.read_handshake_msg(&msg)?;
                            let noise = noise.into_transport_mode()?;
                            let framed = stream.framed(MessagesCodec::new(noise));
                            Ok(framed)
                        })
                })
        });

        Box::new(framed)
    }

    pub fn write_bad_handshake_msg(
        noise: &mut NoiseWrapper,
        current_step: u8,
        step: &Step,
    ) -> Box<Future<Item=(usize, Vec<u8>), Error=io::Error>> {
        let res = match step {
            Step::One(cs, size) | Step::Two(cs, size) if *cs == current_step => {
                Ok((*size, vec![0; *size]))
            }
            _ => noise.write_handshake_msg()
        };

        Box::new(done(res.map_err(|e| e.into())))
    }
}

