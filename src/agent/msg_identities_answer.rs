use crate::codec::*;
use crate::keys::*;

#[derive(Debug, PartialEq)]
pub struct MsgIdentitiesAnswer {
    pub identities: Vec<(PublicKey, String)>,
}

impl MsgIdentitiesAnswer {
    pub const MSG_NUMBER: u8 = 12;
}

impl Encode for MsgIdentitiesAnswer {
    fn size(&self) -> usize {
        std::mem::size_of::<u8>() + Encode::size(&self.identities)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Encode::encode(&self.identities, e);
    }
}

impl<'a> DecodeRef<'a> for MsgIdentitiesAnswer {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(Self::MSG_NUMBER)?;
        Self {
            identities: DecodeRef::decode(d)?,
        }
        .into()
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::algorithm::*;
    use num::BigUint;

    #[test]
    fn test_01() {
        let mut input: Vec<u8> = vec![
            12, 0, 0, 0, 2, 0, 0, 1, 23, 0, 0, 0, 7, 115, 115, 104, 45, 114, 115, 97, 0, 0, 0, 3,
            1, 0, 1, 0, 0, 1, 1, 0, 194, 205, 87, 61, 222, 98, 233, 30, 34, 253, 109, 213, 213, 97,
            222, 36, 72, 88, 115, 31, 74, 221, 121, 156, 5, 47, 55, 10, 182, 96, 170, 136, 69, 113,
            220, 202, 90, 109, 199, 64, 18, 2, 122, 139, 191, 218, 30, 147, 113, 188, 25, 192, 209,
            188, 141, 174, 147, 1, 218, 123, 161, 49, 213, 196, 221, 165, 187, 117, 211, 95, 201,
            32, 209, 144, 234, 6, 206, 106, 200, 74, 157, 26, 55, 169, 113, 169, 42, 228, 52, 123,
            56, 170, 113, 241, 67, 176, 217, 113, 162, 218, 120, 58, 235, 193, 137, 23, 29, 233,
            95, 44, 28, 148, 217, 25, 89, 223, 154, 171, 86, 145, 176, 206, 182, 219, 126, 117, 56,
            84, 77, 18, 145, 225, 220, 245, 122, 134, 14, 205, 72, 245, 181, 230, 15, 57, 206, 71,
            97, 61, 183, 125, 163, 128, 142, 16, 1, 8, 175, 176, 143, 163, 238, 111, 208, 136, 223,
            104, 168, 148, 238, 141, 148, 246, 55, 2, 61, 255, 176, 202, 172, 156, 245, 221, 212,
            189, 118, 210, 53, 220, 116, 175, 158, 178, 255, 84, 228, 215, 213, 204, 108, 145, 29,
            223, 231, 78, 109, 111, 53, 228, 74, 40, 190, 94, 10, 82, 29, 29, 35, 43, 12, 53, 133,
            109, 67, 189, 128, 194, 212, 29, 46, 191, 77, 245, 173, 51, 22, 47, 163, 62, 119, 19,
            5, 118, 156, 56, 201, 152, 103, 32, 107, 32, 100, 110, 91, 155, 156, 215, 0, 0, 0, 42,
            47, 117, 115, 114, 47, 108, 105, 98, 47, 120, 56, 54, 95, 54, 52, 45, 108, 105, 110,
            117, 120, 45, 103, 110, 117, 47, 111, 112, 101, 110, 115, 99, 45, 112, 107, 99, 115,
            49, 49, 46, 115, 111, 0, 0, 0, 51, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53,
            53, 49, 57, 0, 0, 0, 32, 111, 31, 72, 196, 30, 64, 80, 99, 68, 115, 76, 34, 71, 49,
            174, 174, 178, 182, 197, 240, 88, 108, 167, 36, 126, 242, 16, 190, 192, 165, 40, 63, 0,
            0, 0, 12, 114, 115, 115, 104, 45, 101, 120, 97, 109, 112, 108, 101,
        ];
        let mut dec = BDecoder(&mut input[..]);
        let actual: Option<MsgIdentitiesAnswer> = DecodeRef::decode(&mut dec);
        let expected = Some(MsgIdentitiesAnswer {
            identities: vec![
                (
                    PublicKey::RsaPublicKey(SshRsaPublicKey {
                        public_e: BigUint::new(vec![65537]),
                        public_n: BigUint::new(vec![
                            1536924887, 1797284974, 3382208288, 91659320, 2738779923, 2905806383,
                            784289269, 2160251933, 2238530493, 590023733, 173153565, 1244184158,
                            1836004836, 501213006, 3586944145, 4283753687, 1957666482, 1993487836,
                            4124955837, 2966072476, 922893823, 4002256118, 3748178068, 4000305288,
                            2947583907, 2383413512, 3078464384, 3460784445, 3051753273, 248334581,
                            3707075206, 1293062625, 2121611348, 2966337243, 2594920081, 3642317279,
                            1596726420, 2299993577, 2017127361, 3648103130, 1911636912, 880490666,
                            1906911972, 2635741097, 3463104586, 3515935238, 3546269984, 3718626165,
                            2704397764, 2466372219, 3518795182, 1908152768, 3218742931, 302152331,
                            1517143872, 1165089994, 3059788424, 86980362, 1256028572, 1213756191,
                            3579960868, 587034069, 3731024158, 3268237117,
                        ]),
                    }),
                    "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so".into(),
                ),
                (
                    PublicKey::Ed25519PublicKey(Ed25519PublicKey([
                        111, 31, 72, 196, 30, 64, 80, 99, 68, 115, 76, 34, 71, 49, 174, 174, 178,
                        182, 197, 240, 88, 108, 167, 36, 126, 242, 16, 190, 192, 165, 40, 63,
                    ])),
                    "rssh-example".into(),
                ),
            ],
        });
        assert_eq!(actual, expected);
    }
}
