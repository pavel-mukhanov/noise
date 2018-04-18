extern crate futures;
extern crate tokio;
extern crate tokio_core;
extern crate tokio_io;

use std::thread;
use std::time::Duration;
use futures::{Future, Poll, Stream};
use tokio::prelude::*;
use tokio::io::copy;
use tokio_io::{AsyncRead, codec::LinesCodec};
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tokio::net::TcpListener;

// Задача простая
// Сделать клиент и сервер и обмениваться между ними сообщениями
// Как тест хандлер в экзонуме
// Все это должно работать на токио
// потом прикрутить Noise
//

use std::net::SocketAddr;

struct AsWeGetIt<R>(R);

impl<R> Stream for AsWeGetIt<R>
where
    R: AsyncRead + std::fmt::Debug,
{
    type Item = String;
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        println!("self 0 {:?}", self.0);
        //       Ok(String::new())
        Ok(futures::Async::Ready(Some(String::new())))
        //self.0
        //    .read_buf(&mut buf)
        //  .map(|async| async.map(|_| Some(buf)))
    }
}

struct Node {}

impl Node {
    fn spawn(&mut self, listen_addr: &SocketAddr) {
        let addr = listen_addr.clone();
        let thread = thread::spawn(move || {
            Node::start_listen(&addr);
        });
    }

    fn start_listen(addr: &SocketAddr) {
        let listener = TcpListener::bind(addr).expect("err");

        let server = listener
            .incoming()
            .map_err(|e| eprintln!("accept failed = {:?}", e))
            .for_each(|sock| {
                let (writer, reader) = sock.framed(LinesCodec::new()).split();

                // тут должен быть request handler
                // и еще нужно научиться на определенные сообщения
                // писать во writer
                let processor = reader
                .for_each(|bytes| {
                    println!("bytes: {:?}", bytes);
                    Ok(())
                })
                // After our copy operation is complete we just print out some helpful
                // information.
                .and_then(|()| {
                    println!("Socket received FIN packet and closed connection");
                    Ok(())
                })
                .or_else(|err| {
                    println!("Socket closed with error: {:?}", err);
                    // We have to return the error to catch it in the next ``.then` call
                    Err(err)
                })
                .then(|result| {
                    println!("Socket closed with result: {:?}", result);
                    Ok(())
                });

                let sender = writer.send(String::from("msg"));

                tokio::spawn(processor)
            });

        tokio::run(server);
    }
}

fn send_message(message: &str, addr: &SocketAddr) {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let mut stream = TcpStream::connect(&addr, &handle)
        .and_then(|sock| {
            println!("connected to {:?}", sock);
            let (writer, reader) = sock.framed(LinesCodec::new()).split();
            writer.send(message.to_string())
        })
        .map_err(|e| eprintln!("Error: {}", e));

    core.run(stream).unwrap();
}

fn main() {
    // let n1 = Node(ListenPort)
    // let n2 = Node(ListenPort)
    //
    // n1.connect()
    // Handshake
    // n1.send()
    // n1.receive()
    // c.send()
    //
    // Transport
    // c.send()
    //
    // TODO: первая задача это сделать ноду
    // 1. Нода должна слушать сообщения
    // пока мы ее не вы выключим
    // заспаунить ноду в отдельном треде

    println!("nodes!");

    let mut node = Node {};

    let addr = "127.0.0.1:8080".parse().unwrap();

    node.spawn(&addr);
    println!("node spawned!");

    send_message("one", &addr);
    thread::sleep(Duration::from_secs(1));
    send_message("two", &addr);
    thread::sleep(Duration::from_secs(1));
}
