use super::method::*;
use super::msg_userauth_request::*;
use crate::algorithm::*;
use crate::codec::*;
use crate::transport::SessionId;

pub struct SignatureData<'a, S: AuthenticationAlgorithm> {
    pub session_id: &'a SessionId,
    pub user_name: &'a str,
    pub service_name: &'a str,
    pub public_key: S::Identity,
}

impl <'a, S: AuthenticationAlgorithm> Encode for SignatureData<'a, S>
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
        e.push_u8(MsgUserAuthRequest::<PublicKeyMethod<S>>::MSG_NUMBER);
        Encode::encode(&self.user_name, e);
        Encode::encode(&self.service_name, e);
        Encode::encode(&<PublicKeyMethod<S> as AuthMethod>::NAME, e);
        e.push_u8(true as u8);
        Encode::encode(&S::NAME, e);
        Encode::encode(&self.public_key, e);
    }
}

/*
impl<'a, S: SignatureAlgorithm> DecodeRef<'a> for SignatureData<'a, S>
where
    S::PublicKey: DecodeRef<'a>,
    S::Signature: DecodeRef<'a>,
{
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let session_id = DecodeRef::decode(d)?;
        d.take_u8()
            .filter(|x| *x == MsgUserAuthRequest::<PublicKeyMethod<S>>::MSG_NUMBER)?;
        let user_name = DecodeRef::decode(d)?;
        let service_name = DecodeRef::decode(d)?;
        let _: &str = DecodeRef::decode(d).filter(|x| *x == <PublicKeyMethod<S> as Method>::NAME)?;
        d.take_u8().filter(|x| *x != 0)?;
        let _: &str = DecodeRef::decode(d).filter(|x| *x == S::NAME)?;
        let public_key = DecodeRef::decode(d)?;
        Self {
            session_id,
            user_name,
            service_name,
            public_key,
        }
        .into()
    }
}
*/
