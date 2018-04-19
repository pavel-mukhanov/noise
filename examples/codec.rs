use bytes::BytesMut;
use byteorder::{ByteOrder, LittleEndian};
use tokio_io::codec::{Decoder, Encoder};
use snow::Session;

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
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {

        let data = buf.split_to(2).to_vec();
        println!("buf {:?}", data);

        let mut read_to = vec![0u8; 65535];

        let len = self.session.read_message(&buf, &mut read_to).unwrap();

        let res =  String::from_utf8_lossy(&read_to[..len]);
        Ok(Some(res.to_string()))
    }
}

impl Encoder for MessageCodec {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        Ok(())
    }
}