use crate::algorithm::*;
use crate::codec::*;
use crate::codec_ssh::*;
use crate::language::*;

#[derive(Debug,Clone,Copy)]
pub struct KexCookie ([u8;16]);

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
    pub fn new(cookie: KexCookie) -> Self {
        Self {
            cookie: cookie,
            kex_algorithms: vec![KexAlgorithm::Curve25519Sha256AtLibsshDotOrg],
            server_host_key_algorithms: vec![HostKeyAlgorithm::SshEd25519],
            encryption_algorithms_client_to_server: vec![EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom],
            encryption_algorithms_server_to_client: vec![EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom],
            mac_algorithms_client_to_server: vec![],
            mac_algorithms_server_to_client: vec![],
            compression_algorithms_client_to_server: vec![],
            compression_algorithms_server_to_client: vec![],
            languages_client_to_server: vec![],
            languages_server_to_client: vec![],
            first_packet_follows: false
        }
    }
}

impl <'a> SshCodec<'a> for KexInit {
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
    fn encode(&self, c: &mut Encoder<'a>) {
        c.push_u8(20);
        c.push_bytes(&self.cookie.0[..]);
        NameList::encode(&self.kex_algorithms,c);
        NameList::encode(&self.server_host_key_algorithms,c);
        NameList::encode(&self.encryption_algorithms_client_to_server,c);
        NameList::encode(&self.encryption_algorithms_server_to_client,c);
        NameList::encode(&self.mac_algorithms_client_to_server,c);
        NameList::encode(&self.mac_algorithms_server_to_client,c);
        NameList::encode(&self.compression_algorithms_client_to_server,c);
        NameList::encode(&self.compression_algorithms_server_to_client,c);
        NameList::encode(&self.languages_client_to_server,c);
        NameList::encode(&self.languages_server_to_client,c);
        c.push_u8(self.first_packet_follows as u8);
        c.push_u32be(0);
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        c.take_u8().and_then(|x| if x == 20 { Some(()) } else { None })?;
        let r = Self {
            cookie: KexCookie({ let mut x = [0;16]; c.take_bytes_into(&mut x)?; x }),
            kex_algorithms: NameList::decode(c)?,
            server_host_key_algorithms: NameList::decode(c)?,
            encryption_algorithms_client_to_server: NameList::decode(c)?,
            encryption_algorithms_server_to_client: NameList::decode(c)?,
            mac_algorithms_client_to_server: NameList::decode(c)?,
            mac_algorithms_server_to_client: NameList::decode(c)?,
            compression_algorithms_client_to_server: NameList::decode(c)?,
            compression_algorithms_server_to_client: NameList::decode(c)?,
            languages_client_to_server: NameList::decode(c)?,
            languages_server_to_client: NameList::decode(c)?,
            first_packet_follows: c.take_u8().map(|x| x != 0)?,
        };
        c.take_u32be()?;
        r.into()
    }
}
