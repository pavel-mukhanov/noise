extern crate bytes;
extern crate exonum;
extern crate noise;
extern crate snow;
extern crate tokio_io;

use bytes::{BufMut, BytesMut};
use noise::noise_codec::NoiseCodec;
use snow::*;
use snow::params::*;
use snow::types::*;
use snow::wrappers::crypto_wrapper::Dh25519;
use snow::wrappers::rand_wrapper::RandomOs;
use tokio_io::codec::{Decoder, Encoder};

const MSG_SIZE: usize = 4096;

#[test]
fn test_noise_codec_long_msg() {
    static PATTERN: &'static str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
    static SECRET: &'static [u8] = b"secret secret secret key secrets";

    let mut static_i: Dh25519 = Default::default();
    let mut static_r: Dh25519 = Default::default();

    let mut rand = RandomOs::default();
    static_i.generate(&mut rand);
    static_r.generate(&mut rand);

    let mut h_i = NoiseBuilder::new(PATTERN.parse().unwrap())
        .local_private_key(static_i.privkey())
        .remote_public_key(static_i.pubkey())
//        .psk(3, &SECRET)
        .build_initiator()
        .unwrap();

    let mut h_r = NoiseBuilder::new(PATTERN.parse().unwrap())
        .local_private_key(static_r.privkey())
        .remote_public_key(static_r.pubkey())
//        .psk(3, &SECRET)
        .build_responder()
        .unwrap();

    let mut buffer_msg = [0u8; MSG_SIZE * 2];
    let mut buffer_out = [0u8; MSG_SIZE * 2];

    // get the handshaking out of the way for even testing
    let len = h_i.write_message(&[0u8; 0], &mut buffer_msg).unwrap();
    h_r.read_message(&buffer_msg[..len], &mut buffer_out)
        .unwrap();
    let len = h_r.write_message(&[0u8; 0], &mut buffer_msg).unwrap();
    h_i.read_message(&buffer_msg[..len], &mut buffer_out)
        .unwrap();
    let len = h_i.write_message(&[0u8; 0], &mut buffer_msg).unwrap();
    h_r.read_message(&buffer_msg[..len], &mut buffer_out)
        .unwrap();

    let h_i = h_i.into_transport_mode().unwrap();
    let h_r = h_r.into_transport_mode().unwrap();
    let mut buf = BytesMut::with_capacity(MSG_SIZE * 2);
    let mut codec_i = NoiseCodec::new(h_i);
    let mut codec_r = NoiseCodec::new(h_r);

//    let bytes = vec![0u8; 10];
    let bytes = vec![0u8; 65551];

    let s = String::from_utf8(bytes).unwrap();

    codec_i.encode(s.clone(), &mut buf);
    let decoded_s = codec_r.decode(&mut buf).unwrap().unwrap();

    assert_eq!(s, decoded_s);
}
