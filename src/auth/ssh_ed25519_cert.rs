use super::*;
use crate::util::check;
use crate::util::cidr::Cidr;
use crate::util::codec::*;
use std::net::IpAddr;
use std::time::SystemTime;

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

#[derive(Clone, Debug, PartialEq)]
pub struct SshEd25519Cert {
    nonce: [u8; 32],
    pk: [u8; 32],
    serial: u64,
    type_: u32,
    key_id: String,
    valid_principals: Vec<String>,
    valid_after: u64,
    valid_before: u64,
    critical_options: Vec<(String, String)>,
    extensions: Vec<(String, String)>,
    reserved: Vec<u8>,
    authority: PublicKey,
    signature: Signature,
}

impl SshEd25519Cert {
    const TYPE_USER: u32 = 1;
    const TYPE_HOST: u32 = 2;

    const OPT_FORCE_COMMAND: &'static str = "force-command";
    const OPT_SOURCE_ADDRESS: &'static str = "source-address";

    pub const NAME: &'static str = "ssh-ed25519-cert-v01@openssh.com";

    pub fn pk(&self) -> &[u8; 32] {
        &self.pk
    }

    pub fn is_valid_principal(&self, principal: &str) -> bool {
        self.valid_principals.is_empty()
            || self
                .valid_principals
                .iter()
                .find(|x| *x == principal)
                .is_some()
    }

    pub fn is_valid_period(&self) -> bool {
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.valid_after <= current_time && current_time < self.valid_before
    }

    pub fn is_valid_options(&self) -> bool {
        // Short-cut if not options are present
        if self.critical_options.is_empty() {
            return true;
        }
        // Determine whether each option occors at most once
        let mut opts: Vec<&str> = self.critical_options.iter().map(|x| x.0.as_str()).collect();
        let n0 = opts.len();
        opts.sort();
        opts.dedup();
        let n1 = opts.len();
        if n0 != n1 {
            return false;
        }
        // Check whether all options are known to this implementation
        if self.type_ == Self::TYPE_USER {
            let f = |x: &&str| *x == Self::OPT_FORCE_COMMAND || *x == Self::OPT_SOURCE_ADDRESS;
            opts.iter().all(f)
        } else {
            false
        }
    }

    pub fn is_valid_source(&self, src: &IpAddr) -> bool {
        let f = |(x, _): &&(String, String)| x == Self::OPT_SOURCE_ADDRESS;
        if let Some((_, cidrs)) = self.critical_options.iter().find(f) {
            for cidr in cidrs.split(',') {
                if Cidr(cidr).contains(src) {
                    return true;
                }
            }
            false
        } else {
            true
        }
    }

    pub fn is_valid_ca_signature(&self) -> bool {
        let key = &self.authority;
        let data = SliceEncoder::encode(self);
        data.get(..data.len() - Encode::size(&self.signature))
            .and_then(|data| self.signature.verify(key, data).ok())
            .is_some()
    }
}

impl Cert for SshEd25519Cert {
    fn authority(&self) -> &Identity {
        &self.authority
    }
    fn validate_as_host(&self, hostname: &str) -> Result<(), CertError> {
        check(self.type_ == Self::TYPE_HOST).ok_or(CertError::InvalidType)?;
        check(self.is_valid_principal(hostname)).ok_or(CertError::InvalidPrincipal)?;
        check(self.is_valid_period()).ok_or(CertError::InvalidPeriod)?;
        check(self.is_valid_options()).ok_or(CertError::InvalidOptions)?;
        check(self.is_valid_ca_signature()).ok_or(CertError::InvalidSignature)?;
        Ok(())
    }
    fn validate_as_client(&self, username: &str, source: &IpAddr) -> Result<(), CertError> {
        check(self.type_ == Self::TYPE_USER).ok_or(CertError::InvalidType)?;
        check(self.is_valid_principal(username)).ok_or(CertError::InvalidPrincipal)?;
        check(self.is_valid_period()).ok_or(CertError::InvalidPeriod)?;
        check(self.is_valid_options()).ok_or(CertError::InvalidOptions)?;
        check(self.is_valid_source(source)).ok_or(CertError::InvalidSource)?;
        check(self.is_valid_ca_signature()).ok_or(CertError::InvalidSignature)?;
        Ok(())
    }
}

impl Encode for SshEd25519Cert {
    fn size(&self) -> usize {
        let mut n = 0;
        n += 4 + SshEd25519Cert::NAME.len();
        n += 4 + self.nonce.len();
        n += 4 + self.pk.len();
        n += 8 + 4;
        n += 4 + self.key_id.len();
        n += ListRef(&self.valid_principals).size();
        n += 8 + 8;
        n += ListRef(&self.critical_options).size();
        n += ListRef(&self.extensions).size();
        n += 4 + self.reserved.len();
        n += self.authority.size();
        n += self.signature.size();
        n
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_str_framed(SshEd25519Cert::NAME)?;
        e.push_bytes_framed(&self.nonce)?;
        e.push_bytes_framed(&self.pk)?;
        e.push_u64be(self.serial)?;
        e.push_u32be(self.type_)?;
        e.push_str_framed(&self.key_id)?;
        e.push(&ListRef(&self.valid_principals))?;
        e.push_u64be(self.valid_after)?;
        e.push_u64be(self.valid_before)?;
        e.push(&ListRef(&self.critical_options))?;
        e.push(&ListRef(&self.extensions))?;
        e.push_bytes_framed(&self.reserved)?;
        e.push(&self.authority)?;
        e.push(&self.signature)
    }
}

impl Decode for SshEd25519Cert {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let _: &str = DecodeRef::decode(c).filter(|x| *x == SshEd25519Cert::NAME)?;
        Self {
            nonce: {
                c.expect_u32be(32)?;
                let mut x: [u8; 32] = [0; 32];
                c.take_bytes_into(&mut x[..])?;
                x
            },
            pk: {
                c.expect_u32be(32)?;
                let mut x: [u8; 32] = [0; 32];
                c.take_bytes_into(&mut x[..])?;
                x
            },
            serial: c.take_u64be()?,
            type_: c.take_u32be()?,
            key_id: Decode::decode(c)?,
            valid_principals: <List<String> as Decode>::decode(c)?.0,
            valid_after: c.take_u64be()?,
            valid_before: c.take_u64be()?,
            critical_options: <List<(String, String)> as Decode>::decode(c)?.0,
            extensions: <List<(String, String)> as Decode>::decode(c)?.0,
            reserved: {
                let len = c.take_u32be()?;
                Vec::from(c.take_bytes(len as usize)?)
            },
            authority: Decode::decode(c)?,
            signature: Decode::decode(c)?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode() {
        let raw = include_bytes!("../../resources/ed25519-user-cert.pub.raw");
        let _: SshEd25519Cert = SliceDecoder::decode(raw).unwrap();
    }

    #[test]
    fn test_decode_encode_decode_eq() {
        let r1 = include_bytes!("../../resources/ed25519-user-cert.pub.raw");
        let c1: SshEd25519Cert = SliceDecoder::decode(r1).unwrap();
        let r2 = SliceEncoder::encode(&c1);
        let c2: SshEd25519Cert = SliceDecoder::decode(&r2).unwrap();

        assert_eq!(c1.nonce, c2.nonce);
        assert_eq!(c1.pk, c2.pk);
        assert_eq!(c1.serial, c2.serial);
        assert_eq!(c1.type_, c2.type_);
        assert_eq!(c1.key_id, c2.key_id);
        assert_eq!(c1.valid_principals, c2.valid_principals);
        assert_eq!(c1.valid_after, c2.valid_after);
        assert_eq!(c1.valid_before, c2.valid_before);
        assert_eq!(c1.critical_options, c2.critical_options);
        assert_eq!(c1.extensions, c2.extensions);
        assert_eq!(c1.reserved, c2.reserved);
        assert_eq!(c1.authority, c2.authority);
        assert_eq!(c1.signature, c2.signature);
    }

    #[test]
    fn test_signature_valid() {
        let raw = include_bytes!("../../resources/ed25519-user-cert.pub.raw");
        let crt: SshEd25519Cert = SliceDecoder::decode(raw).unwrap();
        assert_eq!(crt.is_valid_ca_signature(), true);
    }

    #[test]
    fn test_signature_invalid() {
        let raw = include_bytes!("../../resources/ed25519-user-cert.pub.raw");
        let mut crt: SshEd25519Cert = SliceDecoder::decode(raw).unwrap();
        crt.nonce[0] += 1;
        assert_eq!(crt.is_valid_ca_signature(), false);
    }
}
