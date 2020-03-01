use super::*;
use crate::codec::*;
use crate::message::*;

#[derive(Debug, Clone, PartialEq)]
pub struct MsgKexInit<T = String> {
    pub cookie: KexCookie,
    pub kex_algorithms: Vec<T>,
    pub server_host_key_algorithms: Vec<T>,
    pub encryption_algorithms_client_to_server: Vec<T>,
    pub encryption_algorithms_server_to_client: Vec<T>,
    pub mac_algorithms_client_to_server: Vec<T>,
    pub mac_algorithms_server_to_client: Vec<T>,
    pub compression_algorithms_client_to_server: Vec<T>,
    pub compression_algorithms_server_to_client: Vec<T>,
    pub languages_client_to_server: Vec<T>,
    pub languages_server_to_client: Vec<T>,
    pub first_packet_follows: bool,
}

impl<T: Clone> MsgKexInit<T> {
    pub fn new(
        cookie: KexCookie,
        kex_algorithms: Vec<T>,
        server_host_key_algorithms: Vec<T>,
        encryption_algorithms: Vec<T>,
        mac_algorithms: Vec<T>,
        compression_algorithms: Vec<T>,
    ) -> Self {
        Self {
            cookie: cookie,
            kex_algorithms,
            server_host_key_algorithms,
            encryption_algorithms_client_to_server: encryption_algorithms.clone(),
            encryption_algorithms_server_to_client: encryption_algorithms,
            mac_algorithms_client_to_server: mac_algorithms.clone(),
            mac_algorithms_server_to_client: mac_algorithms,
            compression_algorithms_client_to_server: compression_algorithms.clone(),
            compression_algorithms_server_to_client: compression_algorithms,
            languages_client_to_server: vec![],
            languages_server_to_client: vec![],
            first_packet_follows: false,
        }
    }
}

impl<T> Message for MsgKexInit<T> {
    const NUMBER: u8 = 20;
}

impl<T: AsRef<[u8]>> Encode for MsgKexInit<T> {
    fn size(&self) -> usize {
        1 + 16
            + 1
            + 4
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
        e.push_u8(<Self as Message>::NUMBER);
        e.push_bytes(&self.cookie);
        NameList::encode(&self.kex_algorithms, e);
        NameList::encode(&self.server_host_key_algorithms, e);
        NameList::encode(&self.encryption_algorithms_client_to_server, e);
        NameList::encode(&self.encryption_algorithms_server_to_client, e);
        NameList::encode(&self.mac_algorithms_client_to_server, e);
        NameList::encode(&self.mac_algorithms_server_to_client, e);
        NameList::encode(&self.compression_algorithms_client_to_server, e);
        NameList::encode(&self.compression_algorithms_server_to_client, e);
        NameList::encode(&self.languages_client_to_server, e);
        NameList::encode(&self.languages_server_to_client, e);
        e.push_u8(self.first_packet_follows as u8);
        e.push_u32be(0);
    }
}

impl Decode for MsgKexInit {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        let r = Self {
            cookie: KexCookie({
                let mut x = [0; 16];
                d.take_into(&mut x)?;
                x
            }),
            kex_algorithms: NameList::decode_string(d)?,
            server_host_key_algorithms: NameList::decode_string(d)?,
            encryption_algorithms_client_to_server: NameList::decode_string(d)?,
            encryption_algorithms_server_to_client: NameList::decode_string(d)?,
            mac_algorithms_client_to_server: NameList::decode_string(d)?,
            mac_algorithms_server_to_client: NameList::decode_string(d)?,
            compression_algorithms_client_to_server: NameList::decode_string(d)?,
            compression_algorithms_server_to_client: NameList::decode_string(d)?,
            languages_client_to_server: NameList::decode_string(d)?,
            languages_server_to_client: NameList::decode_string(d)?,
            first_packet_follows: d.take_u8().map(|x| x != 0)?,
        };
        d.take_u32be()?;
        r.into()
    }
}

#[cfg(test)]
impl From<MsgKexInit<&'static str>> for MsgKexInit {
    fn from(x: MsgKexInit<&'static str>) -> Self {
        let f = |y: Vec<&'static str>| y.into_iter().map(Into::into).collect();
        Self {
            cookie: x.cookie,
            kex_algorithms: x.kex_algorithms.into_iter().map(Into::into).collect(),
            server_host_key_algorithms: f(x.server_host_key_algorithms),
            encryption_algorithms_client_to_server: f(x.encryption_algorithms_client_to_server),
            encryption_algorithms_server_to_client: f(x.encryption_algorithms_server_to_client),
            mac_algorithms_client_to_server: f(x.mac_algorithms_client_to_server),
            mac_algorithms_server_to_client: f(x.mac_algorithms_server_to_client),
            compression_algorithms_client_to_server: f(x.compression_algorithms_client_to_server),
            compression_algorithms_server_to_client: f(x.compression_algorithms_server_to_client),
            languages_client_to_server: f(x.languages_client_to_server),
            languages_server_to_client: f(x.languages_server_to_client),
            first_packet_follows: x.first_packet_follows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_01() {
        let msg1: MsgKexInit<String> = MsgKexInit::new(
            KexCookie([0; 16]),
            vec!["kex".into()],
            vec!["hk".into()],
            vec!["enc1".into()],
            vec!["mac1".into()],
            vec!["comp1".into()],
        );
        let msg2 = MsgKexInit {
            cookie: KexCookie([0; 16]),
            kex_algorithms: vec!["kex".into()],
            server_host_key_algorithms: vec!["hk".into()],
            encryption_algorithms_client_to_server: vec!["enc1".into()],
            encryption_algorithms_server_to_client: vec!["enc1".into()],
            mac_algorithms_client_to_server: vec!["mac1".into()],
            mac_algorithms_server_to_client: vec!["mac1".into()],
            compression_algorithms_client_to_server: vec!["comp1".into()],
            compression_algorithms_server_to_client: vec!["comp1".into()],
            languages_client_to_server: vec![],
            languages_server_to_client: vec![],
            first_packet_follows: false,
        };
        assert_eq!(msg1, msg2);
    }

    #[test]
    fn test_encode_01() {
        let msg: MsgKexInit<String> = MsgKexInit {
            cookie: KexCookie([0; 16]),
            kex_algorithms: vec!["kex".into()],
            server_host_key_algorithms: vec!["hk".into()],
            encryption_algorithms_client_to_server: vec!["enc1".into()],
            encryption_algorithms_server_to_client: vec!["enc2".into()],
            mac_algorithms_client_to_server: vec!["mac1".into()],
            mac_algorithms_server_to_client: vec!["mac2".into()],
            compression_algorithms_client_to_server: vec!["comp1".into()],
            compression_algorithms_server_to_client: vec!["comp2".into()],
            languages_client_to_server: vec!["lang1".into()],
            languages_server_to_client: vec!["lang2".into(), "lang3".into()],
            first_packet_follows: false,
        };
        let expected: [u8; 109] = [
            20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 107, 101, 120, 0, 0, 0,
            2, 104, 107, 0, 0, 0, 4, 101, 110, 99, 49, 0, 0, 0, 4, 101, 110, 99, 50, 0, 0, 0, 4,
            109, 97, 99, 49, 0, 0, 0, 4, 109, 97, 99, 50, 0, 0, 0, 5, 99, 111, 109, 112, 49, 0, 0,
            0, 5, 99, 111, 109, 112, 50, 0, 0, 0, 5, 108, 97, 110, 103, 49, 0, 0, 0, 11, 108, 97,
            110, 103, 50, 44, 108, 97, 110, 103, 51, 0, 0, 0, 0, 0,
        ];
        let actual = BEncoder::encode(&msg);
        assert_eq!(&expected[..], &actual[..]);
    }

    #[test]
    fn test_decode_01() {
        let msg = MsgKexInit {
            cookie: KexCookie([0; 16]),
            kex_algorithms: vec!["kex".into()],
            server_host_key_algorithms: vec!["hk".into()],
            encryption_algorithms_client_to_server: vec!["enc1".into()],
            encryption_algorithms_server_to_client: vec!["enc2".into()],
            mac_algorithms_client_to_server: vec!["mac1".into()],
            mac_algorithms_server_to_client: vec!["mac2".into()],
            compression_algorithms_client_to_server: vec!["comp1".into()],
            compression_algorithms_server_to_client: vec!["comp2".into()],
            languages_client_to_server: vec!["lang1".into()],
            languages_server_to_client: vec!["lang2".into(), "lang3".into()],
            first_packet_follows: false,
        };
        let bin: [u8; 109] = [
            20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 107, 101, 120, 0, 0, 0,
            2, 104, 107, 0, 0, 0, 4, 101, 110, 99, 49, 0, 0, 0, 4, 101, 110, 99, 50, 0, 0, 0, 4,
            109, 97, 99, 49, 0, 0, 0, 4, 109, 97, 99, 50, 0, 0, 0, 5, 99, 111, 109, 112, 49, 0, 0,
            0, 5, 99, 111, 109, 112, 50, 0, 0, 0, 5, 108, 97, 110, 103, 49, 0, 0, 0, 11, 108, 97,
            110, 103, 50, 44, 108, 97, 110, 103, 51, 0, 0, 0, 0, 0,
        ];
        assert_eq!(msg, BDecoder::decode(&bin[..]).unwrap());
    }
}
