use super::KexInit;
use crate::transport::identification::*;
use crate::keys::*;
use crate::codec::*;
use crate::codec_ssh::*;

use sha2::{Sha256, Digest};

pub struct KexHash<'a> {
    pub client_identification: &'a Identification,
    pub server_identification: &'a Identification,
    pub client_kex_init: &'a KexInit,
    pub server_kex_init: &'a KexInit,
    pub server_host_key: &'a PublicKey,
    pub dh_client_key: &'a [u8],
    pub dh_server_key: &'a [u8],
    pub dh_secret: &'a [u8],
}

impl <'a> KexHash<'a> {
    fn size(&'a self) -> usize {
        SshCodec::size(self.client_identification) + 2
        + SshCodec::size(self.server_identification) + 2
        + 4 + SshCodec::size(self.client_kex_init)
        + 4 + SshCodec::size(self.server_kex_init)
        + SshCodec::size(self.server_host_key)
        + 4 + self.dh_client_key.len()
        + 4 + self.dh_server_key.len()
        + 4 + self.dh_secret.len()
    }

    fn encode(&self, c: &mut Encoder) {
        SshCodec::encode(self.client_identification, c);
        c.push_u8(0x0d);
        c.push_u8(0x0a);
        SshCodec::encode(self.server_identification, c);
        c.push_u8(0x0d);
        c.push_u8(0x0a);
        c.push_u32be(SshCodec::size(self.client_kex_init) as u32);
        SshCodec::encode(self.client_kex_init, c);
        c.push_u32be(SshCodec::size(self.server_kex_init) as u32);
        SshCodec::encode(self.server_kex_init, c);
        SshCodec::encode(self.server_host_key, c);
        c.push_u32be(self.dh_client_key.len() as u32);
        c.push_bytes(self.dh_client_key);
        c.push_u32be(self.dh_server_key.len() as u32);
        c.push_bytes(self.dh_server_key);
        c.push_u32be(self.dh_secret.len() as u32);
        c.push_bytes(self.dh_secret);
    }

    pub fn as_sha256_digest(&self) -> [u8;32] {
        let size = self.size();
        let mut vec = Vec::with_capacity(size);
        vec.resize(size, 0);
        self.encode(&mut Encoder::from(&mut vec[..]));
        let mut digest = [0;32];
        digest.copy_from_slice(Sha256::digest(&vec).as_slice());
        digest
    }
}
