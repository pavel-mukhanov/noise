use bytes::BytesMut;
use byteorder::{ByteOrder, LittleEndian};
use tokio_io::codec::{Decoder, Encoder};
use snow::Session;
use snow::transportstate::TransportState;

use std::io;

pub struct MessageCodec {
    max_message_len: u32,
    session: Session,
}

impl  MessageCodec {
    pub fn new( session: Session) -> Self {
        MessageCodec { max_message_len: 1024 , session }
    }
}

impl Decoder for MessageCodec {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {

        println!("buf {:?}", buf);
        Ok(None)
    }
}

impl Encoder for MessageCodec {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        Ok(())
    }
}