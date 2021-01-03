use super::*;
use crate::util::check;
use crate::util::cidr::Cidr;
use crate::util::codec::*;
use std::convert::TryInto;
use std::net::IpAddr;
use std::time::SystemTime;

/// See <https://cvsweb.openbsd.org/src/usr.bin/ssh/PROTOCOL.certkeys>.
#[derive(Clone, Debug, PartialEq)]
pub struct SshEd25519Cert {
    nonce: Vec<u8>,
    pk: [u8; 32],
    serial: u64,
    type_: CertType,
    key_id: String,
    valid_principals: Vec<String>,
    valid_after: u64,
    valid_before: u64,
    critical_options: Vec<CertOption>,
    extensions: Vec<CertExtension>,
    reserved: Vec<u8>,
    authority: PublicKey,
    signature: Signature,
}

impl SshEd25519Cert {
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
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()
            .map(|x| x.as_secs())
            .filter(|now| self.valid_after <= *now && *now < self.valid_before)
            .is_some()
    }

    pub fn is_valid_options(&self) -> bool {
        let mut force_command = 0;
        let mut source_address = 0;
        let mut other = 0;
        for x in &self.critical_options {
            match x {
                CertOption::ForceCommand(_) => force_command += 1,
                CertOption::SourceAddress(_) => source_address += 1,
                CertOption::Other(_, _) => other += 1,
            }
        }
        force_command <= 1 && source_address <= 1 && other == 0
    }

    pub fn is_valid_source(&self, src: &IpAddr) -> bool {
        for x in &self.critical_options {
            if let CertOption::SourceAddress(s) = x {
                for cidr in s.split(',') {
                    if Cidr(cidr).contains(src) {
                        return true;
                    }
                }
                return false;
            }
        }
        true
    }

    pub fn is_valid_ca_signature(&self) -> bool {
        if let Some(size) = SshCodec::size(&self.signature) {
            if let Some(data) = SshCodec::encode(self) {
                if let Some(data) = data.get(..data.len() - size) {
                    return self.signature.verify(&self.authority, data).is_ok();
                }
            }
        }
        false
    }
}

impl Cert for SshEd25519Cert {
    fn authority(&self) -> &Identity {
        &self.authority
    }
    fn verify_for_host(&self, hostname: &str) -> Result<(), CertError> {
        check(self.type_ == CertType::HOST).ok_or(CertError::InvalidType)?;
        check(self.is_valid_principal(hostname)).ok_or(CertError::InvalidPrincipal)?;
        check(self.is_valid_period()).ok_or(CertError::InvalidPeriod)?;
        check(self.is_valid_options()).ok_or(CertError::InvalidOptions)?;
        check(self.is_valid_ca_signature()).ok_or(CertError::InvalidSignature)?;
        Ok(())
    }
    fn verify_for_client(&self, username: &str, source: &IpAddr) -> Result<(), CertError> {
        check(self.type_ == CertType::USER).ok_or(CertError::InvalidType)?;
        check(self.is_valid_principal(username)).ok_or(CertError::InvalidPrincipal)?;
        check(self.is_valid_period()).ok_or(CertError::InvalidPeriod)?;
        check(self.is_valid_options()).ok_or(CertError::InvalidOptions)?;
        check(self.is_valid_source(source)).ok_or(CertError::InvalidSource)?;
        check(self.is_valid_ca_signature()).ok_or(CertError::InvalidSignature)?;
        Ok(())
    }
}

impl SshEncode for SshEd25519Cert {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_str_framed(SshEd25519Cert::NAME)?;
        e.push_bytes_framed(&self.nonce)?;
        e.push_bytes_framed(&self.pk)?;
        e.push_u64be(self.serial)?;
        e.push_u32be(self.type_.0)?;
        e.push_str_framed(&self.key_id)?;
        e.push_list(&self.valid_principals)?;
        e.push_u64be(self.valid_after)?;
        e.push_u64be(self.valid_before)?;
        e.push_list(&self.critical_options)?;
        e.push_list(&self.extensions)?;
        e.push_bytes_framed(&self.reserved)?;
        e.push(&self.authority)?;
        e.push(&self.signature)
    }
}

impl SshDecode for SshEd25519Cert {
    fn decode<'a, D: SshDecoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_str_framed(SshEd25519Cert::NAME)?;
        Some(Self {
            nonce: c.take_bytes_framed()?.into(),
            pk: c.take_bytes_framed()?.try_into().ok()?,
            serial: c.take_u64be()?,
            type_: c.take_u32be().map(CertType)?,
            key_id: c.take_str_framed()?.into(),
            valid_principals: c.take_list()?,
            valid_after: c.take_u64be()?,
            valid_before: c.take_u64be()?,
            critical_options: c.take_list()?,
            extensions: c.take_list()?,
            reserved: c.take_bytes_framed()?.into(),
            authority: c.take()?,
            signature: c.take()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_ed25519_cert_decode_user() {
        let raw = include_bytes!("../../resources/ed25519-user-cert.pub.raw");
        let crt: SshEd25519Cert = SshCodec::decode(raw).unwrap();
        assert_eq!(crt.nonce.len(), 32);
        assert_eq!(crt.nonce[0], 202);
        assert_eq!(crt.pk[0], 217);
        assert_eq!(crt.type_, CertType::USER);
        assert_eq!(crt.key_id, "cert1");
        assert_eq!(crt.valid_principals, vec!["user1", "user2"]);
        assert_eq!(crt.valid_after, 1608329580);
        assert_eq!(crt.valid_before, 2817929649);
        assert_eq!(crt.critical_options.len(), 2);
        assert_eq!(
            crt.critical_options[0],
            CertOption::ForceCommand("ls".into())
        );
        assert_eq!(
            crt.critical_options[1],
            CertOption::SourceAddress("10.0.0.0/16,127.0.0.1/32".into())
        );
        assert_eq!(crt.extensions.len(), 5);
        assert_eq!(crt.extensions[0], CertExtension::PermitX11Forwarding);
        assert_eq!(crt.extensions[1], CertExtension::PermitAgentForwarding);
        assert_eq!(crt.extensions[2], CertExtension::PermitPortForwarding);
        assert_eq!(crt.extensions[3], CertExtension::PermitPty);
        assert_eq!(crt.extensions[4], CertExtension::PermitUserRc);
        assert_eq!(crt.reserved, vec![]);
    }

    #[test]
    fn ssh_ed25519_cert_decode_host() {
        let raw = include_bytes!("../../resources/ed25519-host-cert.pub.raw");
        let crt: SshEd25519Cert = SshCodec::decode(raw).unwrap();
        assert_eq!(crt.nonce.len(), 32);
        assert_eq!(crt.nonce[0], 112);
        assert_eq!(crt.pk[0], 119);
        assert_eq!(crt.type_, CertType::HOST);
        assert_eq!(crt.key_id, "cert2");
        assert_eq!(
            crt.valid_principals,
            vec!["foo.example.com", "bar.example.com"]
        );
        assert_eq!(crt.valid_after, 1608329580);
        assert_eq!(crt.valid_before, 2817929649);
        assert_eq!(crt.critical_options.len(), 0);
        assert_eq!(crt.extensions.len(), 0);
        assert_eq!(crt.reserved, vec![]);
    }

    #[test]
    fn ssh_ed25519_cert_decode_encode_decode_eq() {
        let raw1 = include_bytes!("../../resources/ed25519-user-cert.pub.raw");
        let crt1: SshEd25519Cert = SshCodec::decode(raw1).unwrap();
        let raw2 = SshCodec::encode(&crt1).unwrap();
        let crt2: SshEd25519Cert = SshCodec::decode(&raw2).unwrap();

        assert_eq!(crt1.nonce, crt2.nonce);
        assert_eq!(crt1.pk, crt2.pk);
        assert_eq!(crt1.serial, crt2.serial);
        assert_eq!(crt1.type_, crt2.type_);
        assert_eq!(crt1.key_id, crt2.key_id);
        assert_eq!(crt1.valid_principals, crt2.valid_principals);
        assert_eq!(crt1.valid_after, crt2.valid_after);
        assert_eq!(crt1.valid_before, crt2.valid_before);
        assert_eq!(crt1.critical_options, crt2.critical_options);
        assert_eq!(crt1.extensions, crt2.extensions);
        assert_eq!(crt1.reserved, crt2.reserved);
        assert_eq!(crt1.authority, crt2.authority);
        assert_eq!(crt1.signature, crt2.signature);
    }

    #[test]
    fn ssh_ed25519_cert_signature_valid() {
        let raw = include_bytes!("../../resources/ed25519-user-cert.pub.raw");
        let crt: SshEd25519Cert = SshCodec::decode(raw).unwrap();
        assert_eq!(crt.is_valid_ca_signature(), true);
    }

    #[test]
    fn ssh_ed25519_cert_signature_invalid() {
        let raw = include_bytes!("../../resources/ed25519-user-cert.pub.raw");
        let mut crt: SshEd25519Cert = SshCodec::decode(raw).unwrap();
        crt.nonce[0] += 1;
        assert_eq!(crt.is_valid_ca_signature(), false);
    }
}
