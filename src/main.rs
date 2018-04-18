extern crate bytes;
extern crate clap;
extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate snow;
extern crate tokio;
extern crate tokio_core;
extern crate tokio_io;

use futures::{Future, Poll, Stream};
use tokio::prelude::*;
use tokio::io::copy;
use tokio_io::AsyncRead;
use bytes::BytesMut;
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tokio::net::TcpListener;
use clap::App;
use snow::{Error, NoiseBuilder};
use snow::params::NoiseParams;
use tokio_io::codec::LinesCodec;

static SECRET: &'static [u8] = b"i don't care for fidget spinners";
lazy_static! {
    static ref PARAMS: NoiseParams = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
}

fn main() {
    let matches = App::new("noise")
        .args_from_usage("-s --server 'Server mode'")
        .get_matches();

    if matches.is_present("server") {
        run_server();
    } else {
        run_client();
    }
}

fn run_server() -> Result<(), Error> {
    let addr = "127.0.0.1:12345".parse().unwrap();
    let listener = TcpListener::bind(&addr).expect("unable to bind TCP listener");

    // Pull out a stream of sockets for incoming connections
    let server = listener
        .incoming()
        .for_each(|sock| {
            let (writer, reader) = sock.framed(LinesCodec::new()).split();


            Ok(())
        })
        .map_err(|e| eprintln!("accept failed = {:?}", e));

    // Start the Tokio runtime
    tokio::run(server);
    Ok(())
}

//fn server_handshake(reader:

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

    let addr = "127.0.0.1:12345".parse().unwrap();
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    // Connect to our server, which is hopefully listening.
    let mut stream = TcpStream::connect(&addr, &handle)
        .and_then(|socket| {
            let (writer, reader) = socket.framed(LinesCodec::new()).split();
            Ok(())
        })
        .map_err(|e| eprintln!("Error: {}", e));

    core.run(stream).unwrap();
}
