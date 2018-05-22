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

#[macro_use]
extern crate failure;

#[macro_use]
extern crate env_logger;

#[macro_use]
extern crate log;

use futures::future::Future;
use futures::Sink;
use futures::Stream;
use snow::NoiseBuilder;
use snow::params::NoiseParams;
use std::error::Error as StdError;
use std::io;
use tokio_core::net::TcpStream;
use tokio_io::AsyncRead;
use std::time::SystemTime;
use noise_codec::MessagesCodec;
use tokio::executor::current_thread;
use tokio_io::codec::Framed;

pub mod wrapper;
pub mod noise_main;
pub mod noise_codec;


