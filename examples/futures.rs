extern crate futures;
extern crate tokio_core;

use futures::prelude::*;
use futures::future::ok;
use futures::done;
use futures::stream::Stream;
use std::thread;
use std::error::Error;
use tokio_core::reactor::{Core, Handle};
use std::time::{Duration, SystemTime};
use futures::sync::mpsc;
use futures::sync::mpsc::Sender;

type FutResult = Box<Future<Item=u32, Error=Box<Error>>>;
type PlainResult = Result<u32, Box<Error>>;

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let (rx, tx) = mpsc::channel(10);

    let fut = rx.send(1).and_then(|x| x.send(2));

    let fut2 = tx.for_each(|item| {
        println!("item {}", item);
        Ok(())
    });

    core.run(fut
        .map_err(|e| {
            ()
        })
        .and_then(|x| {
            fut2
        })
    );
}

fn incoming() {

}

fn squared(i: u32) -> FutResult {
    thread::sleep(Duration::from_secs(2));
    Box::new(ok(i * i))
}

fn add(i: u32, n: u32) -> FutResult {
    Box::new(ok(i + n))
}

fn plain_add(i: u32, n: u32) -> PlainResult {
    Ok(i + n)
}

struct MyStream {
    current: u32,
    max: u32,
}

impl MyStream {
    pub fn new(max: u32) -> MyStream {
        MyStream {
            current: 0,
            max: max,
        }
    }
}

impl Stream for MyStream {
    type Item = u32;
    type Error = Box<Error>;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        use futures::future::Executor;

        match self.current {
            ref mut x if *x < self.max => {
                *x = *x + 1;

                Ok(Async::Ready(Some(*x)))
            }
            _ => Ok(Async::Ready(None)),
        }
    }
}
