use bytes::BytesMut;
use byteorder::{ByteOrder, BigEndian};
use tokio_io::codec::{Decoder, Encoder};
use snow::Session;

use std::io;

#[allow(dead_code)]
pub struct MessageCodec {
    max_message_len: u32,
    session: Session,
}

impl MessageCodec {
    pub fn new(session: Session) -> Self {
        MessageCodec {
            max_message_len: 1024,
            session,
        }
    }
}

impl Decoder for MessageCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {
        if buf.len() < 2 {
            return Ok(None);
        };

        let len = BigEndian::read_u16(buf) as usize;
        let data = buf.split_to(len + 2).to_vec();
        let data = &data[2..];
        let mut read_to = vec![0u8; len];
        let len = self.session.read_message(data, &mut read_to).unwrap();
        let res = String::from_utf8_lossy(&read_to[..len]);
        Ok(Some(res.to_string()))
    }
}

impl Encoder for MessageCodec {
    type Item = String;
    type Error = io::Error;

    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        let mut tmp_buf = vec![0u8; 65535];
        let len = self.session
            .write_message(msg.as_bytes(), &mut tmp_buf)
            .unwrap();
        let mut msg_len_buf = vec![(len >> 8) as u8, (len & 0xff) as u8];
        let tmp_buf = &tmp_buf[0..len];
        msg_len_buf.extend_from_slice(tmp_buf);

        println!("sending to socket {:?}", msg_len_buf);

        buf.extend_from_slice(&msg_len_buf);
        Ok(())
    }
}
