use super::method::*;
use super::msg_userauth_request::*;
use crate::algorithm::*;
use crate::codec::*;
use crate::message::*;
use crate::transport::SessionId;

pub struct SignatureData<'a, S: AuthenticationAlgorithm> {
    pub session_id: &'a SessionId,
    pub user_name: &'a str,
    pub service_name: &'a str,
    pub public_key: S::Identity,
}

impl<'a, S: AuthenticationAlgorithm> Encode for SignatureData<'a, S>
where
    S::Identity: Encode,
    S::Signature: Encode,
{
    fn size(&self) -> usize {
        Encode::size(&self.session_id)
            + 1
            + Encode::size(&self.user_name)
            + Encode::size(&self.service_name)
            + Encode::size(&<PublicKeyMethod<S> as AuthMethod>::NAME)
            + 1
            + Encode::size(&S::NAME)
            + Encode::size(&self.public_key)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Encode::encode(&self.session_id, e);
        e.push_u8(<MsgUserAuthRequest<PublicKeyMethod<S>> as Message>::NUMBER);
        Encode::encode(&self.user_name, e);
        Encode::encode(&self.service_name, e);
        Encode::encode(&<PublicKeyMethod<S> as AuthMethod>::NAME, e);
        e.push_u8(true as u8);
        Encode::encode(&S::NAME, e);
        Encode::encode(&self.public_key, e);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::algorithm::authentication::*;

    #[test]
    fn test_encode_01() {
        let x: SignatureData<SshEd25519> = SignatureData {
            session_id: &SessionId::new([1; 32]),
            user_name: "user",
            service_name: "service",
            public_key: SshEd25519PublicKey([2; 32]),
        };
        let actual = BEncoder::encode(&x);
        let expected = [
            0, 0, 0, 32, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 50, 0, 0, 0, 4, 117, 115, 101, 114, 0, 0, 0, 7, 115, 101, 114,
            118, 105, 99, 101, 0, 0, 0, 9, 112, 117, 98, 108, 105, 99, 107, 101, 121, 1, 0, 0, 0,
            11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 51, 0, 0, 0, 11, 115,
            115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        ];
        assert_eq!(&actual[..], &expected[..]);
    }
}
