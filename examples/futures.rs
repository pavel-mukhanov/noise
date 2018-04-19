extern crate futures;
extern crate tokio_core;
extern crate snow;
#[macro_use]
extern crate lazy_static;

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
use snow::NoiseBuilder;
use snow::params::NoiseParams;
use futures::future::FutureResult;
use snow::Session;

type FutResult = Box<Future<Item=u32, Error=Box<Error>>>;
type PlainResult = Result<u32, Box<Error>>;


static SECRET: &'static [u8] = b"i don't care for fidget spinners";
lazy_static! {
    static ref PARAMS: NoiseParams = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
}

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let builder: NoiseBuilder = NoiseBuilder::new(PARAMS.clone());
    let static_key = builder.generate_private_key().unwrap();
    let mut noise = builder
        .local_private_key(&static_key)
        .psk(3, SECRET)
        .build_initiator()
        .unwrap();

    let fut =
        squared(20).map(|x| Ok(noise)).and_then(|n:Result<Session, ()>| {
            let noise = n.unwrap();
            ok(())
        });

    core.run(fut);
}

fn incoming() {}

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
