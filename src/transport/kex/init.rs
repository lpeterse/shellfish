use crate::algorithm::*;
use crate::codec::*;
use crate::language::*;
use super::*;

#[derive(Debug,Clone)]
pub struct KexInit {
    pub cookie: KexCookie,
    pub kex_algorithms: Vec<KexAlgorithm>,
    pub server_host_key_algorithms: Vec<HostKeyAlgorithm>,
    pub encryption_algorithms_client_to_server: Vec<EncryptionAlgorithm>,
    pub encryption_algorithms_server_to_client: Vec<EncryptionAlgorithm>,
    pub mac_algorithms_client_to_server: Vec<MacAlgorithm>,
    pub mac_algorithms_server_to_client: Vec<MacAlgorithm>,
    pub compression_algorithms_client_to_server: Vec<CompressionAlgorithm>,
    pub compression_algorithms_server_to_client: Vec<CompressionAlgorithm>,
    pub languages_client_to_server: Vec<Language>,
    pub languages_server_to_client: Vec<Language>,
    pub first_packet_follows: bool
}

impl KexInit {
    pub const MSG_NUMBER: u8 = 20;

    pub fn new(cookie: KexCookie) -> Self {
        Self {
            cookie: cookie,
            kex_algorithms: vec![KexAlgorithm::Curve25519Sha256AtLibsshDotOrg],
            server_host_key_algorithms: vec![HostKeyAlgorithm::SshEd25519],
            encryption_algorithms_client_to_server: vec![EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom],
            encryption_algorithms_server_to_client: vec![EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom],
            mac_algorithms_client_to_server: vec![],
            mac_algorithms_server_to_client: vec![],
            compression_algorithms_client_to_server: vec![CompressionAlgorithm::None],
            compression_algorithms_server_to_client: vec![CompressionAlgorithm::None],
            languages_client_to_server: vec![],
            languages_server_to_client: vec![],
            first_packet_follows: false
        }
    }
}

impl <'a> Codec<'a> for KexInit {
    fn size(&self) -> usize {
        1 + 16 + 1 + 4
        + NameList::size(&self.kex_algorithms)
        + NameList::size(&self.server_host_key_algorithms)
        + NameList::size(&self.encryption_algorithms_client_to_server)
        + NameList::size(&self.encryption_algorithms_server_to_client)
        + NameList::size(&self.mac_algorithms_client_to_server)
        + NameList::size(&self.mac_algorithms_server_to_client)
        + NameList::size(&self.compression_algorithms_client_to_server)
        + NameList::size(&self.compression_algorithms_server_to_client)
        + NameList::size(&self.languages_client_to_server)
        + NameList::size(&self.languages_server_to_client)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER);
        e.push_bytes(&self.cookie);
        NameList::encode(&self.kex_algorithms,e);
        NameList::encode(&self.server_host_key_algorithms,e);
        NameList::encode(&self.encryption_algorithms_client_to_server,e);
        NameList::encode(&self.encryption_algorithms_server_to_client,e);
        NameList::encode(&self.mac_algorithms_client_to_server,e);
        NameList::encode(&self.mac_algorithms_server_to_client,e);
        NameList::encode(&self.compression_algorithms_client_to_server,e);
        NameList::encode(&self.compression_algorithms_server_to_client,e);
        NameList::encode(&self.languages_client_to_server,e);
        NameList::encode(&self.languages_server_to_client,e);
        e.push_u8(self.first_packet_follows as u8);
        e.push_u32be(0);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().and_then(|x| if x == Self::MSG_NUMBER { Some(()) } else { None })?;
        let r = Self {
            cookie: KexCookie({ let mut x = [0;16]; d.take_into(&mut x)?; x }),
            kex_algorithms: NameList::decode(d)?,
            server_host_key_algorithms: NameList::decode(d)?,
            encryption_algorithms_client_to_server: NameList::decode(d)?,
            encryption_algorithms_server_to_client: NameList::decode(d)?,
            mac_algorithms_client_to_server: NameList::decode(d)?,
            mac_algorithms_server_to_client: NameList::decode(d)?,
            compression_algorithms_client_to_server: NameList::decode(d)?,
            compression_algorithms_server_to_client: NameList::decode(d)?,
            languages_client_to_server: NameList::decode(d)?,
            languages_server_to_client: NameList::decode(d)?,
            first_packet_follows: d.take_u8().map(|x| x != 0)?,
        };
        d.take_u32be()?;
        r.into()
    }
}
