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

fn read(sock: TcpStream) -> Box<Future<Item=(TcpStream, Vec<u8>), Error=io::Error>> {
    let buf = vec![0u8; HANDSHAKE_HEADER_LENGTH];
    Box::new(
        read_exact(sock, buf)
            .and_then(|(stream, msg)| read_exact(stream, vec![0u8; msg[0] as usize])),
    )
}

fn write(
    sock: TcpStream,
    buf: &[u8],
    len: usize,
) -> Box<Future<Item=(TcpStream, Vec<u8>), Error=io::Error>> {
    let mut message = vec![0u8; HANDSHAKE_HEADER_LENGTH];
    LittleEndian::write_u16(&mut message, len as u16);
    message.extend_from_slice(&buf[0..len]);
    Box::new(write_all(sock, message))
}

fn read_handshake_msg(
    input: &[u8],
    noise: &mut NoiseWrapper,
) -> Box<Future<Item=(usize, Vec<u8>), Error=io::Error>> {
    let res = noise.read_handshake_msg(input);
    Box::new(done(res.map_err(|e| e.into())))
}

fn write_handshake_msg(
    noise: &mut NoiseWrapper,
) -> Box<Future<Item=(usize, Vec<u8>), Error=io::Error>> {
    let res = noise.write_handshake_msg();
    Box::new(done(res.map_err(|e| e.into())))
}

mod tests {
    // Сначала тест нормального случая
    // Поднимаем TcpListener и коннектимся к нему
    // Соответсвтенно внутри листенера должен быть listen_handshake
    // коннектимся к нему send_handshake'ом
    // собственно и все
    // надо только погасить листенер потом

    use byteorder::{ByteOrder, LittleEndian};
    use env_logger;
    use futures::{Future, Stream};
    use futures::unsync;
    use futures::unsync::oneshot::Receiver;
    use futures::unsync::oneshot::Sender;
    use noise_main::NoiseHandshake;
    use snow::NoiseBuilder;
    use snow::params::NoiseParams;
    use std::error::Error as StdError;
    use std::io::{Read, Write};
    use std::io;
    use std::net::{TcpListener as StdTcpListener, TcpStream as StdTcpStream};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::thread;
    use tokio_core::net::{TcpListener, TcpStream};
    use tokio_core::reactor::Core;
    use wrapper::HandshakeParams;
    use wrapper::NoiseError;
    use snow::Session;
    use noise_main::HandshakeResult;
    use wrapper::NoiseWrapper;
    use noise_codec::MessagesCodec;

    #[derive(PartialEq)]
    pub enum Step {
        One,
        Two,
    }

    #[test]
    fn test_noise_normal_handshake() {
        let addr: SocketAddr = "127.0.0.1:5001".parse().unwrap();
        let addr2 = addr.clone();

        let con = thread::spawn(move || {
            run_server(&addr2)
        });

        let res = send_message(&addr);
        assert!(res.is_ok());
    }

    #[test]
    fn test_noise_plain_listen_handshake() {
        env_logger::init();
        let addr:SocketAddr = "127.0.0.1:5002".parse().unwrap();
        let addr_cloned = addr.clone();
        let con = thread::spawn(move || {
            plain_listen_handshake(Step::One, &addr_cloned)
        });

        thread::sleep_ms(1500);
        let res = send_message(&addr);

        assert!(res.is_err());
    }

    #[test]
    fn test_noise_plain_send_handshake() {
        let addr :SocketAddr= "127.0.0.1:5003".parse().unwrap();
        let addr_cloned = addr.clone();

        let con = thread::spawn(move || {
            run_server(&addr)
        });

        let res = plain_send_handshake(Step::One, &addr);
        assert!(res.is_err());

        let res = plain_send_handshake(Step::Two, &addr);
    }

    fn run_server(addr: &SocketAddr) -> Result<(), ()> {
        let mut core = Core::new().unwrap();
        let handle = core.handle();

        let params = HandshakeParams {
            max_message_len: 1024,
        };

        let fut_stream = TcpListener::bind(addr, &handle).unwrap();
        let fut = fut_stream.incoming()
            .for_each(|(stream, _)| {
                let reader =
                    NoiseHandshake::listen(&params, stream)
                        .and_then(|framed| {
                            Ok(())
                        }).map_err(log_error);

                handle.spawn(reader);
                Ok(())
            })
            .map_err(|e| {
                ()
            });

        core.run(fut)
    }

    fn send_message(addr: &SocketAddr) -> Result<(), io::Error> {
        let mut core = Core::new().unwrap();
        let handle = core.handle();

        let params = HandshakeParams {
            max_message_len: 1024,
        };

        let stream = TcpStream::connect(&addr, &handle)
            .and_then(|sock| {
                NoiseHandshake::send(&params, sock)
                    .and_then(|_| {
                        Ok(())
                    })
            })
            .map_err(into_other);

        core.run(stream)
    }

    pub fn plain_send_handshake(step: Step, addr: &SocketAddr) -> Result<usize, NoiseError> {
        let mut buf = vec![0u8; 65535];
        let params: NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2s".parse().unwrap();

        // Initialize our initiator NoiseSession using a builder.
        let builder: NoiseBuilder = NoiseBuilder::new(params.clone());
        let static_key = builder.generate_private_key().unwrap();
        let mut noise = builder
            .local_private_key(&static_key)
            .build_initiator().unwrap();

        // Connect to our server, which is hopefully listening.
        let mut stream = StdTcpStream::connect(addr).unwrap();

        // -> e
        let len = noise.write_message(&[], &mut buf).unwrap();
        if step == Step::One {
            send(&mut stream, &vec![0u8; len]);
        } else {
            send(&mut stream, &buf[..len]);
        }

        // <- e, ee, s, es
        noise.read_message(&recv(&mut stream).unwrap(), &mut buf)
            .map_err(|e| NoiseError::new(format!("Error while writing noise message: {:?}", e.0)))?;

        // -> s, se
        let len = noise.write_message(&[], &mut buf).unwrap();

        if step == Step::Two {
            send(&mut stream, &vec![0u8; len]);
        } else {
            send(&mut stream, &buf[..len]);
        }
        Ok(0)
    }

    pub fn plain_listen_handshake(step: Step, addr:&SocketAddr) -> Result<usize, NoiseError> {
        let mut buf = vec![0u8; 65535];
        let params: NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2s".parse().unwrap();

        let builder: NoiseBuilder = NoiseBuilder::new(params);
        let static_key = builder.generate_private_key().unwrap();
        let mut noise = builder
            .local_private_key(&static_key)
            .build_responder().unwrap();

        info!("tcp listener bind");
        let (mut stream, _) = StdTcpListener::bind(addr).unwrap().accept().unwrap();

        info!("read message");
        // <- e
        noise.read_message(&recv(&mut stream).unwrap(), &mut buf).unwrap();

        // -> e, ee, s, es
        let len = noise.write_message(&[0u8; 0], &mut buf).unwrap();
        if step == Step::One {
            send(&mut stream, &vec![0u8; len]);
        } else {
            send(&mut stream, &buf[..len]);
        }

        // <- s, se
        noise.read_message(&recv(&mut stream).unwrap(), &mut buf).map_err(|e| NoiseError::new(format!("Error while writing noise message: {:?}", e.0)))?;
        Ok(0)
    }

    fn recv(stream: &mut StdTcpStream) -> io::Result<Vec<u8>> {
        let mut msg_len_buf = vec![0u8; 2];
        stream.read_exact(&mut msg_len_buf)?;
        let len = LittleEndian::read_u16(&msg_len_buf) as usize;
        let mut msg = vec![0u8; len];
        stream.read_exact(&mut msg[..])?;
        Ok(msg)
    }

    fn send(stream: &mut StdTcpStream, buf: &[u8]) {
        let mut msg_len_buf = vec![0u8; 2];
        LittleEndian::write_u16(&mut msg_len_buf, buf.len() as u16);
        msg_len_buf.extend_from_slice(buf);
        stream.write_all(&msg_len_buf).unwrap();
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
}

