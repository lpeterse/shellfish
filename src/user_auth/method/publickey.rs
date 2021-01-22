use super::*;
use crate::identity::*;

#[derive(Debug)]
pub struct PublicKeyMethod {
    pub identity: Identity,
    pub signature: Option<Signature>,
}

impl<'a> AuthMethod for PublicKeyMethod {
    const NAME: &'static str = "publickey";
}

impl SshEncode for PublicKeyMethod {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_bool(self.signature.is_some())?;
        e.push_str_framed(self.identity.algorithm())?;
        e.push(&self.identity)?;
        match self.signature {
            None => Some(()),
            Some(ref x) => e.push(x),
        }
    }
}

impl SshDecode for PublicKeyMethod {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        // FIXME
        // let b = d.take_u8()? != 0;
        // let _: &str = SshDecodeRef::decode(d)?; // FIXME
        // let identity = d.isolate_u32be(|x| SshDecodeRef::decode(x))?;
        // let signature = if b { Some(SshDecodeRef::decode(d)?) } else { None };
        // PublicKeyMethod {
        //     identity,
        //     signature,
        // }
        // .into()

        // let b = d.take_bool()?;
        // SshDecode::decode(d).filter(|x| x == self.identity.algorithm()).map(drop)?;
        // let id = SshDecode::decode(d)?;

        // let len = d.take_u32be()?;
        // let innr = d.take_bytes(len as usize)?;
        // let innr = &mut RefDecoder::new(innr);
        // let algo = SshDecode::decode(innr)?;
        // let data = SshDecodeRef::decode(innr).map(|x: &[u8]| Vec::from(x))?;
        // innr.expect_eoi()?;
        // Some(Self { algo, data })
        panic!()
    }
}

#[cfg(test)]
mod tests {
    // use super::super::super::ssh_ed25519::*;
    // use super::*;

    // #[test]
    // fn test_debug_01() {
    //     let pk = Identity::Ed25519PublicKey(Ed25519PublicKey([2; 32]));
    //     let sg = Signature { algorithm: "ssh-ed25519".into(), signature: vec![3; 64] };
    //     let x: PublicKeyMethod = PublicKeyMethod {
    //         identity: pk,
    //         signature: Some(sg),
    //     };
    //     assert_eq!(format!("{:?}", x), "PublicKeyMethod { identity: Ed25519PublicKey(Ed25519PublicKey([2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2])), signature: Some(Signature { algorithm: \"ssh-ed25519\", signature: [3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3] }) }");
    // }

    // #[test]
    // fn test_encode_01() {
    //     let identity = Identity::Ed25519PublicKey(Ed25519PublicKey([2; 32]));
    //     let x: PublicKeyMethod = PublicKeyMethod {
    //         identity,
    //         signature: None,
    //     };
    //     let actual = SshCodec::encode(&x);
    //     let expected = &[
    //         0, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 51, 0, 0, 0,
    //         11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 2, 2, 2, 2, 2, 2, 2,
    //         2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    //     ][..];
    //     assert_eq!(actual, expected);
    // }

    // #[test]
    // fn test_encode_02() {
    //     let identity = Identity::Ed25519PublicKey(Ed25519PublicKey([2; 32]));
    //     let sg = Signature { algorithm: "ssh-ed25519".into(), signature: vec![3; 64] };
    //     let x = PublicKeyMethod {
    //         identity,
    //         signature: Some(sg),
    //     };
    //     let actual = SshCodec::encode(&x);
    //     let expected = &[
    //         1, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 51, 0, 0, 0,
    //         11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 2, 2, 2, 2, 2, 2, 2,
    //         2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0, 0, 0, 83,
    //         0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 64, 3, 3, 3, 3,
    //         3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
    //         3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
    //         3, 3,
    //     ][..];
    //     assert_eq!(actual, expected);
    // }

    /*
    #[test]
    fn test_decode_01() {
        let identity = Identity::PublicKey(PublicKey::Ed25519(Ed25519PublicKey([2; 32])));
        let x: PublicKeyMethod = SshCodec::decode(
            &[
                0, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 51, 0, 0,
                0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 2, 2, 2, 2, 2,
                2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            ][..],
        )
        .unwrap();
        assert_eq!(x.identity, identity);
        assert_eq!(x.signature, None);
    }

    #[test]
    fn test_decode_02() {
        let identity = Identity::PublicKey(PublicKey::Ed25519(Ed25519PublicKey([2; 32])));
        let sg = Signature::Ed25519(SshEd25519Signature([3; 64]));
        let x: PublicKeyMethod = SshCodec::decode(
            &[
                1, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 51, 0, 0,
                0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 2, 2, 2, 2, 2,
                2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 0,
                0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0,
                64, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
            ][..],
        )
        .unwrap();
        assert_eq!(x.identity, identity);
        assert_eq!(x.signature.unwrap(), sg);
    }
    */
}
