extern crate tokio_core;
extern crate futures;

use tokio_core::reactor::Core;
use std::net::SocketAddr;
use tokio_core::net::TcpListener;
use tokio_core::net::TcpStream;
use futures::future::{Future};
use futures::stream::Stream;

#[test]
fn test_client_server() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let addr: SocketAddr = "127.0.0.1:8000".parse().unwrap();
    let handle_cloned = handle.clone();

    let fut_stream = TcpListener::bind(&addr, &handle_cloned).unwrap();
    let fut = fut_stream.incoming()
        .for_each(move |(stream, _)| {
            println!("connected");
            Ok(())
        })
        .map_err(|e| {
            println!("error ");
        });

    // as before
    handle.spawn(fut);
    let (tx_done, rx_done) = futures::oneshot();
    let stream = TcpStream::connect(&addr, &handle_cloned);
    // once `TcpStream` processing is done, signal completion
    handle.spawn(stream.then(|_| tx_done.send(())));
    // run the loop until completion is signaled
    let res =  core.run(rx_done.then(|_| Ok::<_, ()>(())));
}