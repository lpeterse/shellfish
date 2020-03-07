use super::method::*;
use super::msg_userauth_request::*;
use crate::algorithm::auth::*;
use crate::codec::*;
use crate::message::*;
use crate::transport::SessionId;

/// string    session identifier
/// byte      SSH_MSG_USERAUTH_REQUEST
/// string    user name
/// string    service name
/// string    "publickey"
/// boolean   TRUE
/// string    public key algorithm name
/// string    public key to be used for authentication
pub struct SignatureData<'a> {
    pub session_id: &'a SessionId,
    pub user_name: &'a str,
    pub service_name: &'a str,
    pub identity: &'a Identity,
}

impl<'a> Encode for SignatureData<'a> {
    fn size(&self) -> usize {
        Encode::size(&self.session_id)
            + 1
            + Encode::size(&self.user_name)
            + Encode::size(&self.service_name)
            + Encode::size(&<PublicKeyMethod as AuthMethod>::NAME)
            + 1
            + Encode::size(&self.identity.algorithm())
            + Encode::size(&self.identity)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Encode::encode(&self.session_id, e);
        e.push_u8(<MsgUserAuthRequest<PublicKeyMethod> as Message>::NUMBER);
        Encode::encode(&self.user_name, e);
        Encode::encode(&self.service_name, e);
        Encode::encode(&<PublicKeyMethod as AuthMethod>::NAME, e);
        e.push_u8(true as u8);
        Encode::encode(&self.identity.algorithm(), e);
        Encode::encode(&self.identity, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let x = SignatureData {
            session_id: &SessionId::new([
                41, 47, 231, 244, 246, 141, 145, 191, 204, 234, 29, 219, 118, 44, 26, 47, 205, 64,
                26, 209, 97, 125, 207, 58, 188, 51, 187, 202, 81, 75, 126, 77,
            ]),
            user_name: "lpetersen",
            service_name: "ssh-connection",
            identity: &Identity::PublicKey(PublicKey::Ed25519(SshEd25519PublicKey([
                6, 161, 229, 86, 153, 227, 155, 10, 249, 178, 133, 207, 121, 108, 220, 52, 193,
                161, 162, 243, 150, 202, 192, 242, 222, 166, 188, 190, 158, 169, 52, 114,
            ]))),
        };
        let actual = BEncoder::encode(&x);
        let expected = [
            0, 0, 0, 32, 41, 47, 231, 244, 246, 141, 145, 191, 204, 234, 29, 219, 118, 44, 26, 47,
            205, 64, 26, 209, 97, 125, 207, 58, 188, 51, 187, 202, 81, 75, 126, 77, 50, 0, 0, 0, 9,
            108, 112, 101, 116, 101, 114, 115, 101, 110, 0, 0, 0, 14, 115, 115, 104, 45, 99, 111,
            110, 110, 101, 99, 116, 105, 111, 110, 0, 0, 0, 9, 112, 117, 98, 108, 105, 99, 107,
            101, 121, 1, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 51,
            0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 6, 161, 229,
            86, 153, 227, 155, 10, 249, 178, 133, 207, 121, 108, 220, 52, 193, 161, 162, 243, 150,
            202, 192, 242, 222, 166, 188, 190, 158, 169, 52, 114,
        ];
        assert_eq!(&actual[..], &expected[..]);
    }
}
