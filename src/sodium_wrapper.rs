use rand::{thread_rng, Rng};

use sodiumoxide::crypto::aead::chacha20poly1305 as sodium_chacha20poly1305;
use sodiumoxide::crypto::hash::sha256 as sodium_sha256;
use sodiumoxide::crypto::scalarmult::curve25519 as sodium_curve25519;

// Random data generator.
pub struct SodiumRandom;

pub trait Random {
    fn fill_bytes(&mut self, out: &mut [u8]);
}

impl Default for SodiumRandom {
    fn default() -> SodiumRandom {
        SodiumRandom {}
    }
}

impl Random for SodiumRandom {
    fn fill_bytes(&mut self, out: &mut [u8]) {
        let bytes: Vec<u8> = thread_rng().gen_iter::<u8>().take(out.len()).collect();
        out.copy_from_slice(&bytes);
    }
}

// Elliptic curve 25519.
#[derive(Clone)]
pub struct SodiumDh25519 {
    privkey: sodium_curve25519::Scalar,
    pubkey: sodium_curve25519::GroupElement,
}

impl Default for SodiumDh25519 {
    fn default() -> SodiumDh25519 {
        SodiumDh25519 {
            privkey: sodium_curve25519::Scalar([0; 32]),
            pubkey: sodium_curve25519::GroupElement([0; 32]),
        }
    }
}

impl SodiumDh25519 {
    fn name(&self) -> &'static str {
        "25519"
    }

    fn pub_len(&self) -> usize {
        32
    }

    fn priv_len(&self) -> usize {
        32
    }

    fn set(&mut self, privkey: &[u8]) {
        self.privkey = sodium_curve25519::Scalar::from_slice(privkey)
            .expect("Can't construct private key for Dh25519");
        self.pubkey = sodium_curve25519::scalarmult_base(&self.privkey);
    }

    fn generate(&mut self, rng: &mut Random) {
        let mut privkey_bytes = [0; 32];
        rng.fill_bytes(&mut privkey_bytes);
        privkey_bytes[0] &= 248;
        privkey_bytes[31] &= 127;
        privkey_bytes[31] |= 64;
        self.privkey = sodium_curve25519::Scalar::from_slice(&privkey_bytes)
            .expect("Can't construct private key for Dh25519");
        self.pubkey = sodium_curve25519::scalarmult_base(&self.privkey);
    }

    fn pubkey(&self) -> &[u8] {
        &self.pubkey[0..32]
    }

    fn privkey(&self) -> &[u8] {
        &self.privkey[0..32]
    }

    fn dh(&self, pubkey: &[u8], out: &mut [u8]) {
        let pubkey = sodium_curve25519::GroupElement::from_slice(&pubkey[0..32])
            .expect("Can't construct public key for Dh25519");
        let result =
            sodium_curve25519::scalarmult(&self.privkey, &pubkey).expect("Can't calculate dh");

        out[..32].copy_from_slice(&result[0..32]);
    }
}


#[cfg(test)]
mod tests {
    use sodium_wrapper::SodiumDh25519;
    use sodium_wrapper::{Random, SodiumRandom};
    use sodiumoxide::crypto::sign::{gen_keypair, keypair_from_seed, PublicKey, SecretKey};
    use sodiumoxide::crypto::sign::ed25519::Seed;

    use std::os::raw::c_uchar;
    use std::os::raw::c_int;
    use std::convert::AsMut;
    use snow::NoiseBuilder;

    pub const crypto_sign_ed25519_PUBLICKEYBYTES: usize = 32;
    pub const crypto_sign_ed25519_SECRETKEYBYTES: usize = 64;

    #[link(name = "sodium")]
    extern {
        fn crypto_sign_ed25519_pk_to_curve25519(curve25519_sk: *mut [u8; crypto_sign_ed25519_PUBLICKEYBYTES],
                                                ed25519_sk: *const [u8; crypto_sign_ed25519_PUBLICKEYBYTES]);

        fn crypto_sign_ed25519_sk_to_curve25519(curve25519_sk: *mut [u8; crypto_sign_ed25519_PUBLICKEYBYTES],
                                                ed25519_sk: *const [u8; crypto_sign_ed25519_PUBLICKEYBYTES]);
    }

    fn clone_into_array<A, T>(slice: &[T]) -> A
        where
            A: Default + AsMut<[T]>,
            T: Clone,
    {
        let mut a = Default::default();
        <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
        a
    }

    fn convert_keys_to_curve25519(pk: PublicKey, sk: SecretKey) -> ([u8; 32], [u8; 32]) {
        let mut pk = clone_into_array(&pk[..]);
        let mut sk= clone_into_array(&sk[..32]);

        let mut curve_pk = [0; crypto_sign_ed25519_PUBLICKEYBYTES];
        let mut curve_sk = [0; crypto_sign_ed25519_PUBLICKEYBYTES];
        unsafe {
            crypto_sign_ed25519_pk_to_curve25519(&mut curve_pk, &mut pk);
            crypto_sign_ed25519_sk_to_curve25519(&mut curve_sk, &mut sk);
        }

        (curve_pk, curve_sk)
    }

    #[test]
    fn test_curve25519_dh() {
        // Initiator keys
        let mut dh_i = SodiumDh25519::default();
        let mut random = SodiumRandom::default();
        dh_i.generate(&mut random);
        let dh_cloned_i = dh_i.clone();
        let (public_key_i, secret_key_i) = (dh_cloned_i.pubkey(), dh_cloned_i.privkey());

        // Responder keys
        let mut dh_r = SodiumDh25519::default();
        dh_r.generate(&mut random);
        let dh_cloned_r = dh_r.clone();
        let (public_key_r, secret_key_r) = (dh_cloned_r.pubkey(), dh_cloned_r.privkey());

        let mut output_i = [0u8; 32];
        dh_i.dh(public_key_r, &mut output_i);

        let mut output_r = [0u8; 32];
        dh_r.dh(public_key_i, &mut output_r);

        assert_eq!(output_i, output_r);
    }

    #[test]
    fn test_convert_ed_to_curve_dh() {
        // Generate Ed25519 keys for initiator and responder.
        let (public_key_i, secret_key_i) = gen_keypair();
        let (public_key_r, secret_key_r) = gen_keypair();

        // Convert to Curve25519 keys.
        let (public_key_i, secret_key_i) = convert_keys_to_curve25519(public_key_i, secret_key_i);
        let (public_key_r, secret_key_r) = convert_keys_to_curve25519(public_key_r, secret_key_r);

        // Do DH
        let mut keypair_i: SodiumDh25519 = Default::default();
        keypair_i.set(&secret_key_i[..32]);
        let mut output_i = [0u8; 32];
        keypair_i.dh(public_key_r.as_ref(), &mut output_i);

        let mut keypair_r: SodiumDh25519 = Default::default();
        keypair_r.set(&secret_key_r[..32]);
        let mut output_r = [0u8; 32];
        keypair_r.dh(public_key_i.as_ref(), &mut output_r);

        assert_eq!(output_i, output_r);
    }

    #[test]
    fn test_converted_keys_handshake() {
        const MSG_SIZE: usize = 4096;
        static PATTERN: &'static str = "Noise_XK_25519_ChaChaPoly_BLAKE2s";

        let mut random = SodiumRandom::default();
        let mut seed_bytes = [0; 32];
        random.fill_bytes(&mut seed_bytes);

        let seed = Seed::from_slice(&seed_bytes).unwrap();

        let (public_key_i, secret_key_i) = keypair_from_seed(&seed);
        let (public_key_r, secret_key_r) = keypair_from_seed(&seed);

//        let (public_key_i, secret_key_i) = gen_keypair();
//        let (public_key_r, secret_key_r) = gen_keypair();

//        let public_key_r = &public_key_r[..];
//        let secret_key_i = &secret_key_i[..32];
//        let secret_key_r = &secret_key_r[..32];

        // Convert to Curve25519 keys.
        let (public_key_i, secret_key_i) = convert_keys_to_curve25519(public_key_i, secret_key_i);
        let (public_key_r, secret_key_r) = convert_keys_to_curve25519(public_key_r, secret_key_r);

        let mut h_i = NoiseBuilder::new(PATTERN.parse().unwrap())
            .local_private_key(&secret_key_i)
            .remote_public_key(&public_key_r)
            .build_initiator()
            .unwrap();

        let mut h_r = NoiseBuilder::new(PATTERN.parse().unwrap())
            .local_private_key(&secret_key_r)
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

    }
}
