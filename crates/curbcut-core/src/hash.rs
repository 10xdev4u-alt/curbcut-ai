//! SHA-256 and HMAC-SHA256.
//!
//! A model is pinned by its SHA-256 digest before it is loaded, and the pin is
//! authenticated with an HMAC. Both are implemented here in safe Rust with no
//! dependency, so the check runs identically on every CI runner and on a
//! machine with no network.
//!
//! This module hashes bytes and stores nothing: it has no buffer that outlives
//! a call, so there is no content to zeroize. See
//! [`../../../docs/MODEL_GUARD.md`](../../../docs/MODEL_GUARD.md).

use std::fmt;

/// SHA-256 block size in bytes.
const BLOCK: usize = 64;
/// Digest length in bytes.
pub const DIGEST_BYTES: usize = 32;

/// The 64 round constants: the first 32 bits of the fractional parts of the
/// cube roots of the first 64 primes.
const K: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// A SHA-256 hasher.
///
/// Streaming so a large model file can be fed in chunks without buffering it
/// all. `finish` consumes the hasher.
#[derive(Clone)]
pub struct Sha256 {
    state: [u32; 8],
    buffer: [u8; BLOCK],
    buffered: usize,
    len: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sha256")
            .field("consumed", &self.len)
            .finish_non_exhaustive()
    }
}

impl Sha256 {
    /// Creates a hasher at the start of a message.
    pub fn new() -> Self {
        Self {
            state: [
                0x6a09_e667,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            buffer: [0; BLOCK],
            buffered: 0,
            len: 0,
        }
    }

    /// Feeds bytes into the message.
    pub fn update(&mut self, mut bytes: &[u8]) {
        self.len = self.len.wrapping_add(bytes.len() as u64);
        if self.buffered > 0 {
            let take = (BLOCK - self.buffered).min(bytes.len());
            self.buffer[self.buffered..self.buffered + take].copy_from_slice(&bytes[..take]);
            self.buffered += take;
            bytes = &bytes[take..];
            if self.buffered == BLOCK {
                let block = self.buffer;
                self.compress(&block);
                self.buffered = 0;
            }
        }
        while bytes.len() >= BLOCK {
            let mut block = [0u8; BLOCK];
            block.copy_from_slice(&bytes[..BLOCK]);
            self.compress(&block);
            bytes = &bytes[BLOCK..];
        }
        if !bytes.is_empty() {
            self.buffer[..bytes.len()].copy_from_slice(bytes);
            self.buffered = bytes.len();
        }
    }

    /// Finishes the message and returns the digest.
    pub fn finish(mut self) -> Digest {
        let bit_len = self.len.wrapping_mul(8);
        self.update(&[0x80]);
        while self.buffered != 56 {
            self.update(&[0]);
        }
        // `update` counted the padding, so the length is written from the saved
        // value rather than from the running counter.
        self.update(&bit_len.to_be_bytes());
        let mut out = [0u8; DIGEST_BYTES];
        for (index, word) in self.state.iter().enumerate() {
            out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        Digest(out)
    }

    fn compress(&mut self, block: &[u8; BLOCK]) {
        let mut w = [0u32; 64];
        for index in 0..16 {
            w[index] = u32::from_be_bytes([
                block[index * 4],
                block[index * 4 + 1],
                block[index * 4 + 2],
                block[index * 4 + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

/// A 32-byte digest.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Digest([u8; DIGEST_BYTES]);

impl Digest {
    /// The raw bytes.
    pub fn as_bytes(&self) -> &[u8; DIGEST_BYTES] {
        &self.0
    }

    /// The lowercase hexadecimal form.
    pub fn as_hex(&self) -> String {
        let mut out = String::with_capacity(DIGEST_BYTES * 2);
        for byte in self.0 {
            out.push(hex_digit(byte >> 4));
            out.push(hex_digit(byte & 0x0f));
        }
        out
    }

    /// Parses 64 hexadecimal characters.
    ///
    /// Uppercase is accepted. Returns `None` for any other length or a
    /// non-hex character.
    pub fn from_hex(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != DIGEST_BYTES * 2 {
            return None;
        }
        let mut out = [0u8; DIGEST_BYTES];
        for index in 0..DIGEST_BYTES {
            let high = hex_value(bytes[index * 2])?;
            let low = hex_value(bytes[index * 2 + 1])?;
            out[index] = (high << 4) | low;
        }
        Some(Self(out))
    }

    /// Compares against a hexadecimal digest in constant time.
    ///
    /// The comparison is over the decoded bytes, so an uppercase or lowercase
    /// spelling of the same digest matches.
    pub fn matches_hex(&self, text: &str) -> bool {
        match Self::from_hex(text) {
            Some(other) => {
                let mut diff = 0u8;
                for (a, b) in self.0.iter().zip(other.0.iter()) {
                    diff |= a ^ b;
                }
                diff == 0
            }
            None => false,
        }
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Digest").field(&self.as_hex()).finish()
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_hex())
    }
}

/// Hashes a message.
pub fn sha256(message: &[u8]) -> Digest {
    let mut hasher = Sha256::new();
    hasher.update(message);
    hasher.finish()
}

/// Computes HMAC-SHA256.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> Digest {
    let mut padded = [0u8; BLOCK];
    if key.len() > BLOCK {
        let digest = sha256(key);
        padded[..DIGEST_BYTES].copy_from_slice(digest.as_bytes());
    } else {
        padded[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(&padded.map(|byte| byte ^ 0x36));
    inner.update(message);
    let inner = inner.finish();

    let mut outer = Sha256::new();
    outer.update(&padded.map(|byte| byte ^ 0x5c));
    outer.update(inner.as_bytes());
    outer.finish()
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        _ => (b'a' + (value - 10)) as char,
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_empty_message_matches_the_published_digest() {
        assert_eq!(
            sha256(b"").as_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn abc_matches_the_published_digest() {
        assert_eq!(
            sha256(b"abc").as_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn a_two_block_message_matches_the_published_digest() {
        assert_eq!(
            sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq").as_hex(),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn a_one_million_a_message_matches_the_published_digest() {
        let message = vec![b'a'; 1_000_000];
        assert_eq!(
            sha256(&message).as_hex(),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn streaming_in_any_chunking_gives_the_same_digest() {
        let message: Vec<u8> = (0..=255u8).collect();
        let whole = sha256(&message);
        for chunk in [1usize, 2, 7, 63, 64, 65, 100, 255] {
            let mut hasher = Sha256::new();
            for piece in message.chunks(chunk) {
                hasher.update(piece);
            }
            assert_eq!(hasher.finish(), whole, "chunk size {chunk}");
        }
    }

    #[test]
    fn a_one_bit_change_changes_the_digest() {
        assert_ne!(sha256(b"abc"), sha256(b"abd"));
    }

    #[test]
    fn hex_round_trips_and_rejects_bad_input() {
        let digest = sha256(b"abc");
        let hex = digest.as_hex();
        assert_eq!(Digest::from_hex(&hex), Some(digest));
        assert_eq!(Digest::from_hex(&hex.to_uppercase()), Some(digest));
        assert!(Digest::from_hex("ba7816").is_none(), "too short");
        assert!(Digest::from_hex(&format!("{hex}00")).is_none(), "too long");
        assert!(
            Digest::from_hex(&format!("z{}", &hex[1..])).is_none(),
            "not hexadecimal"
        );
    }

    #[test]
    fn matches_hex_is_case_insensitive() {
        let digest = sha256(b"abc");
        assert!(digest.matches_hex(&digest.as_hex().to_uppercase()));
        assert!(!digest.matches_hex(&"0".repeat(64)));
    }

    #[test]
    fn hmac_matches_rfc_4231_case_1() {
        let key = [0x0b_u8; 20];
        assert_eq!(
            hmac_sha256(&key, b"Hi There").as_hex(),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn hmac_matches_rfc_4231_case_2() {
        assert_eq!(
            hmac_sha256(b"Jefe", b"what do ya want for nothing?").as_hex(),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn hmac_matches_rfc_4231_case_3() {
        let key = [0xaa_u8; 20];
        let message = [0xdd_u8; 50];
        assert_eq!(
            hmac_sha256(&key, &message).as_hex(),
            "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe"
        );
    }

    #[test]
    fn hmac_matches_rfc_4231_case_6_a_key_longer_than_one_block_is_hashed() {
        let key = [0xaa_u8; 131];
        let message = b"Test Using Larger Than Block-Size Key - Hash Key First";
        assert_eq!(
            hmac_sha256(&key, message).as_hex(),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn hmac_changes_with_the_key_and_the_message() {
        let message = b"Hi There";
        assert_ne!(hmac_sha256(b"key", message), hmac_sha256(b"kez", message));
        assert_ne!(
            hmac_sha256(b"key", message),
            hmac_sha256(b"key", b"Hi Therf")
        );
    }
}
