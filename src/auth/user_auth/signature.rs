use super::method::*;
use super::msg_userauth_request::*;
use crate::auth::*;
use crate::transport::Message;
use crate::transport::SessionId;
use crate::util::codec::*;

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
        let mut n = 0;
        n += self.session_id.size();
        n += 1;
        n += 4 + self.user_name.len();
        n += 4 + self.service_name.len();
        n += 4 + <PublicKeyMethod as AuthMethod>::NAME.len();
        n += 1;
        n += 4 + self.identity.algorithm().len();
        n += self.identity.size();
        n
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push(self.session_id)?;
        e.push_u8(<MsgUserAuthRequest<PublicKeyMethod> as Message>::NUMBER)?;
        e.push_str_framed(&self.user_name)?;
        e.push_str_framed(&self.service_name)?;
        e.push_str_framed(<PublicKeyMethod as AuthMethod>::NAME)?;
        e.push_u8(true as u8)?;
        e.push_str_framed(&self.identity.algorithm())?;
        e.push(self.identity)
    }
}

/*
#[cfg(test)]
mod tests {
    use super::super::ssh_ed25519::*;
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
            identity: &Identity::Ed25519PublicKey(Ed25519PublicKey([
                6, 161, 229, 86, 153, 227, 155, 10, 249, 178, 133, 207, 121, 108, 220, 52, 193,
                161, 162, 243, 150, 202, 192, 242, 222, 166, 188, 190, 158, 169, 52, 114,
            ])),
        };
        let actual = SliceEncoder::encode(&x);
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
*/
