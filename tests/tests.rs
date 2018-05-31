extern crate bytes;
extern crate noise;
extern crate snow;
extern crate tokio_io;
extern crate crypto;

use bytes::{BufMut, BytesMut};
use noise::noise_codec::MessagesCodec;
use snow::*;
use snow::params::*;
use snow::types::*;
use snow::wrappers::rand_wrapper::RandomOs;
use tokio_io::codec::{Decoder, Encoder};
use crypto::curve25519::curve25519;
use crypto::curve25519::curve25519_base;

const MSG_SIZE: usize = 4096;


//

#[test]
fn test_noise_codec_long_msg() {
    static PATTERN: &'static str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

    let mut static_i: Dh25519 = Default::default();
    let mut static_r: Dh25519 = Default::default();

    let mut rand = RandomOs::default();
    static_i.generate(&mut rand);
    static_r.generate(&mut rand);

    let pub_i = static_i.pubkey();
    let pub_r = static_r.pubkey();

    let sec_i = static_i.privkey();
    let sec_r = static_r.privkey();

    let mut h_i = NoiseBuilder::with_resolver(PATTERN.parse().unwrap(), Box::new(TestResolver::new()))
//        let mut h_i = NoiseBuilder::new(PATTERN.parse().unwrap())
        .local_private_key(sec_i)
//            .remote_public_key(pub_r)
        .build_initiator()
        .unwrap();

    let mut h_r = NoiseBuilder::with_resolver(PATTERN.parse().unwrap(), Box::new(TestResolver::new()))
        .local_private_key(sec_r)
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
}

#[allow(unused)]
struct TestResolver {
    parent: DefaultResolver,
}

#[allow(unused)]
impl TestResolver {

    pub fn new() -> Self {
        return TestResolver { parent: DefaultResolver {} }
    }
}

impl CryptoResolver for TestResolver {
    fn resolve_rng(&self) -> Option<Box<Random>> {
        self.parent.resolve_rng()
    }

    fn resolve_dh(&self, choice: &DHChoice) -> Option<Box<Dh>> {
        match *choice {
            DHChoice::Curve25519 => Some(Box::new(Dh25519::default())),
            _                    => None,
        }
    }

    fn resolve_hash(&self, choice: &HashChoice) -> Option<Box<Hash>> {
        self.parent.resolve_hash(choice)
    }

    fn resolve_cipher(&self, choice: &CipherChoice) -> Option<Box<Cipher>> {
        self.parent.resolve_cipher(choice)
    }
}

#[derive(Default)]
pub struct Dh25519 {
    privkey: [u8; 32],
    pubkey:  [u8; 32],
}

impl Dh for Dh25519 {

    fn name(&self) -> &'static str {
        static NAME: &'static str = "25519";
        NAME
    }

    fn pub_len(&self) -> usize {
        32
    }

    fn priv_len(&self) -> usize {
        32
    }

    fn set(&mut self, privkey: &[u8]) {
        copy_memory(privkey, &mut self.privkey);

        let pubkey = curve25519_base(&self.privkey);

        copy_memory(&pubkey, &mut self.pubkey);
    }

    fn generate(&mut self, rng: &mut Random) {
        rng.fill_bytes(&mut self.privkey);
        self.privkey[0]  &= 248;
        self.privkey[31] &= 127;
        self.privkey[31] |= 64;
        let pubkey = curve25519_base(&self.privkey);
        copy_memory(&pubkey, &mut self.pubkey);
    }

    fn pubkey(&self) -> &[u8] {
        &self.pubkey
    }

    fn privkey(&self) -> &[u8] {
        &self.privkey
    }

    fn dh(&self, pubkey: &[u8], out: &mut [u8]) {
        let result = curve25519(&self.privkey, pubkey);
        copy_memory(&result, out);
    }

}

pub fn copy_memory(input: &[u8], out: &mut [u8]) -> usize {
    for count in 0..input.len() {out[count] = input[count];}
    input.len()
}

//#[test]
//fn test_secret_from_seed() {
//
//    let mut seed = vec![0u8; 32];
//
//    for i in 0..32u8 {
//        seed[i as usize] = i;
//    }
//
//    println!("seed {:?}", seed);
//
//
//    let (public_key, secret_key) = gen_keypair_from_seed(&Seed::new(from_slice(seed.as_slice())));
//
//    println!("secret_key {:?}", &secret_key[..64]);
//
//
//    let mut static_i: Dh25519 = Default::default();
//    let mut static_r: Dh25519 = Default::default();
//
//    let mut rand = RandomOs::default();
//    static_i.generate(&mut rand);
//    static_r.generate(&mut rand);
//
//    let pub_i = static_i.pubkey();
//    let pub_r = static_r.pubkey();
//
//    let sec_i = static_i.privkey();
//    let sec_r = static_r.privkey();
//
//
//    println!("pub_i {:?}, sec_i {:?}", pub_i, sec_i);
//}
//
//fn from_slice(bytes: &[u8]) -> [u8; 32] {
//    let mut array = [0; 32];
//    let bytes = &bytes[..array.len()]; // panics if not enough data
//    array.copy_from_slice(bytes);
//    array
//}