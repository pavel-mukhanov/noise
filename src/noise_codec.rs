use byteorder::{BigEndian,LittleEndian, ByteOrder};
use bytes::BytesMut;
use snow::Session;
use std::io;
use tokio_io::codec::{Decoder, Encoder};

#[allow(dead_code)]
pub struct NoiseCodec {
    max_message_len: u32,
    session: Session,
}

impl NoiseCodec {
    pub fn new(session: Session) -> Self {
        NoiseCodec {
            max_message_len: 1024,
            session,
        }
    }
}

impl Decoder for NoiseCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {
        if buf.len() < 2 {
            return Ok(None);
        };

        let len = LittleEndian::read_u32(buf) as usize;
        println!("message len {}", len);

        let data = buf.split_to(len + 4).to_vec();
        let data = &data[4..];
        let mut readed_data = vec![0u8; 0];
        let mut readed_len = 0usize;

        data.chunks(65535).for_each(|chunk| {
            let mut read_to = vec![0u8; chunk.len()];
            println!("chunk len {:?}", chunk.len());
            readed_len += self.session.read_message(chunk, &mut read_to).unwrap();
            readed_data.extend_from_slice(&read_to);
        });

        let res = String::from_utf8_lossy(&readed_data[..readed_len]);
        Ok(Some(res.to_string()))
    }
}

impl Encoder for NoiseCodec {
    type Item = String;
    type Error = io::Error;

    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        let mut len = 0usize;

        let mut write_to_buf = vec![0u8; 0];

        msg.as_bytes().chunks(65535 - 16).for_each(|chunk| {
            let mut tmp_buf = vec![0u8; 65535];
            len += self.session
                .write_message( chunk,&mut tmp_buf)
                .unwrap();
            println!("written_bytes {:?}", len);
            write_to_buf.extend_from_slice(&tmp_buf);
        });

        println!("sending to socket len {}", len);
        let mut msg_len_buf = vec![0u8; 4];
        LittleEndian::write_u32(&mut msg_len_buf, len as u32);
        let write_to_buf = &write_to_buf[0..len];
        msg_len_buf.extend_from_slice(write_to_buf);
        buf.extend_from_slice(&msg_len_buf);
        Ok(())
    }
}
