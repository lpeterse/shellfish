use super::*;
use crate::codec::*;

/*
ED25519 certificate

    string    "ssh-ed25519-cert-v01@openssh.com"
    string    nonce
    string    pk
    uint64    serial
    uint32    type
    string    key id
    string    valid principals
    uint64    valid after
    uint64    valid before
    string    critical options
    string    extensions
    string    reserved
    string    signature key
    string    signature
*/

pub struct SshEd25519Cert {}

impl SshEd25519Cert {
    const NAME: &'static str = "ssh-ed25519-cert-v01@openssh.com";
}

impl AuthenticationAlgorithm for SshEd25519Cert {
    type Identity = SshEd25519Certificate;
    type Signature = SshEd25519Signature;
    type SignatureFlags = SshEd25519SignatureFlags;

    const NAME: &'static str = SshEd25519Cert::NAME;
}

#[derive(Clone, Debug, PartialEq)]
pub struct SshEd25519Certificate {
    nonce: [u8; 32],
    pk: SshEd25519PublicKey,
    serial: u64,
    type_: u32,
    key_id: String,
    valid_principals: Vec<String>,
    valid_after: u64,
    valid_before: u64,
    critical_options: Vec<(String, String)>,
    extensions: Vec<(String, String)>,
    reserved: Vec<u8>,
    signature_key: SshEd25519PublicKey,
    signature: SshEd25519Signature,
}

impl Encode for SshEd25519Certificate {
    fn size(&self) -> usize {
        panic!("FIXME")
    }
    fn encode<E: Encoder>(&self, _: &mut E) {
        panic!("FIXME")
    }
}

impl Decode for SshEd25519Certificate {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be().map(drop)?;
        let _: &str = DecodeRef::decode(c)
            .filter(|x| *x == <SshEd25519Cert as AuthenticationAlgorithm>::NAME)?;
        Self {
            nonce: {
                c.expect_u32be(32)?;
                let mut x: [u8; 32] = [0; 32];
                c.take_into(&mut x[..])?;
                x
            },
            pk: {
                c.expect_u32be(32)?;
                let mut x: [u8; 32] = [0; 32];
                c.take_into(&mut x[..])?;
                SshEd25519PublicKey(x)
            },
            serial: c.take_u64be()?,
            type_: c.take_u32be()?,
            key_id: Decode::decode(c)?,
            valid_principals: {
                let x: List<String> = Decode::decode(c)?;
                x.0
            },
            valid_after: c.take_u64be()?,
            valid_before: c.take_u64be()?,
            critical_options: {
                let x: List<(String, String)> = Decode::decode(c)?;
                x.0
            },
            extensions: {
                let x: List<(String, String)> = Decode::decode(c)?;
                x.0
            },
            reserved: {
                let len = c.take_u32be()?;
                Vec::from(c.take_bytes(len as usize)?)
            },
            signature_key: Decode::decode(c)?,
            signature: Decode::decode(c)?,
        }
        .into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decode_01() {
        let input: [u8; 454] = [
            0, 0, 1, 194, 0, 0, 0, 32, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 45, 99,
            101, 114, 116, 45, 118, 48, 49, 64, 111, 112, 101, 110, 115, 115, 104, 46, 99, 111,
            109, 0, 0, 0, 32, 161, 204, 42, 130, 20, 70, 115, 37, 164, 38, 13, 36, 146, 18, 52,
            225, 85, 154, 120, 152, 57, 20, 246, 86, 238, 215, 53, 249, 110, 99, 100, 213, 0, 0, 0,
            32, 111, 31, 72, 196, 30, 64, 80, 99, 68, 115, 76, 34, 71, 49, 174, 174, 178, 182, 197,
            240, 88, 108, 167, 36, 126, 242, 16, 190, 192, 165, 40, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 0, 0, 9, 108, 112, 101, 116, 101, 114, 115, 101, 110, 0, 0, 0, 13, 0, 0, 0,
            9, 108, 112, 101, 116, 101, 114, 115, 101, 110, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255,
            255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 130, 0, 0, 0, 21, 112, 101, 114, 109,
            105, 116, 45, 88, 49, 49, 45, 102, 111, 114, 119, 97, 114, 100, 105, 110, 103, 0, 0, 0,
            0, 0, 0, 0, 23, 112, 101, 114, 109, 105, 116, 45, 97, 103, 101, 110, 116, 45, 102, 111,
            114, 119, 97, 114, 100, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 22, 112, 101, 114, 109,
            105, 116, 45, 112, 111, 114, 116, 45, 102, 111, 114, 119, 97, 114, 100, 105, 110, 103,
            0, 0, 0, 0, 0, 0, 0, 10, 112, 101, 114, 109, 105, 116, 45, 112, 116, 121, 0, 0, 0, 0,
            0, 0, 0, 14, 112, 101, 114, 109, 105, 116, 45, 117, 115, 101, 114, 45, 114, 99, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 51, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53,
            49, 57, 0, 0, 0, 32, 6, 161, 229, 86, 153, 227, 155, 10, 249, 178, 133, 207, 121, 108,
            220, 52, 193, 161, 162, 243, 150, 202, 192, 242, 222, 166, 188, 190, 158, 169, 52, 114,
            0, 0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 64,
            16, 137, 11, 71, 101, 117, 195, 117, 243, 253, 86, 164, 12, 163, 30, 233, 24, 28, 19,
            205, 67, 68, 68, 112, 37, 38, 62, 38, 124, 179, 214, 16, 173, 54, 204, 200, 13, 157,
            135, 209, 220, 36, 118, 102, 127, 96, 137, 214, 53, 18, 154, 25, 246, 147, 22, 216,
            123, 174, 142, 141, 199, 36, 188, 1,
        ];
        let cert: SshEd25519Certificate = BDecoder::decode(input.as_ref()).unwrap();
        assert_eq!(
            cert.nonce,
            [
                161, 204, 42, 130, 20, 70, 115, 37, 164, 38, 13, 36, 146, 18, 52, 225, 85, 154,
                120, 152, 57, 20, 246, 86, 238, 215, 53, 249, 110, 99, 100, 213
            ]
        );
        assert_eq!(
            cert.pk,
            SshEd25519PublicKey([
                111, 31, 72, 196, 30, 64, 80, 99, 68, 115, 76, 34, 71, 49, 174, 174, 178, 182, 197,
                240, 88, 108, 167, 36, 126, 242, 16, 190, 192, 165, 40, 63
            ])
        );
        assert_eq!(cert.serial, 0);
        assert_eq!(cert.type_, 1);
        assert_eq!(cert.key_id, "lpetersen");
        assert_eq!(cert.valid_principals, vec!["lpetersen"]);
        assert_eq!(cert.valid_after, 0);
        assert_eq!(cert.valid_before, 18446744073709551615);
        assert_eq!(cert.critical_options, vec![]);
        assert_eq!(
            cert.extensions,
            vec![
                ("permit-X11-forwarding".into(), "".into()),
                ("permit-agent-forwarding".into(), "".into()),
                ("permit-port-forwarding".into(), "".into()),
                ("permit-pty".into(), "".into()),
                ("permit-user-rc".into(), "".into())
            ]
        );
        assert_eq!(cert.reserved, vec![]);
        assert_eq!(
            cert.signature_key,
            SshEd25519PublicKey([
                6, 161, 229, 86, 153, 227, 155, 10, 249, 178, 133, 207, 121, 108, 220, 52, 193,
                161, 162, 243, 150, 202, 192, 242, 222, 166, 188, 190, 158, 169, 52, 114
            ])
        );
        assert_eq!(
            cert.signature,
            SshEd25519Signature([
                16, 137, 11, 71, 101, 117, 195, 117, 243, 253, 86, 164, 12, 163, 30, 233, 24, 28,
                19, 205, 67, 68, 68, 112, 37, 38, 62, 38, 124, 179, 214, 16, 173, 54, 204, 200, 13,
                157, 135, 209, 220, 36, 118, 102, 127, 96, 137, 214, 53, 18, 154, 25, 246, 147, 22,
                216, 123, 174, 142, 141, 199, 36, 188, 1
            ])
        )
    }
}
