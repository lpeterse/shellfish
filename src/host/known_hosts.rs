use super::verification::{HostKeyVerifier, VerificationError, VerificationFuture};
use crate::algorithm::auth::*;
use crate::codec::*;
use crate::util::*;
use crate::util::glob::*;

use async_std::fs::File;
use async_std::io::{BufReader, Read};
use async_std::prelude::*;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct KnownHosts {
    paths: Vec<PathBuf>,
}

impl KnownHosts {
    /// Loop through all files and lines until either a match or a revocation has been found.
    pub async fn verify(
        &self,
        name: &str,
        identity: &Identity,
    ) -> Result<(), VerificationError> {
        for path in &self.paths {
            log::debug!("Looking for {} in {:?}", name, path);
            match Self::verify_path(&path, name, identity).await {
                Err(VerificationError::FileError(std::io::ErrorKind::NotFound)) => continue,
                Err(VerificationError::KeyNotFound) => continue,
                Ok(()) => return Ok(()),
                e => return e,
            }
        }
        Err(VerificationError::KeyNotFound)
    }

    pub async fn verify_path(
        path: &PathBuf,
        name: &str,
        identity: &Identity,
    ) -> Result<(), VerificationError> {
        let e = |e: std::io::Error| VerificationError::FileError(e.kind());
        let file = File::open(path).await.map_err(e)?;
        Self::verify_file(file, name, identity).await
    }

    pub async fn verify_file<T: Read + Unpin>(
        file: T,
        name: &str,
        identity: &Identity,
    ) -> Result<(), VerificationError> {
        let mut lines = BufReader::new(file).lines().enumerate();
        while let Some(line) = lines.next().await {
            if let (i, Ok(line)) = line {
                if let Some(line) = Line::parse(&line) {
                    match line.verify(name, identity) {
                        Ok(()) => {
                            log::debug!("Line {}: Found matching host key", i + 1);
                            return Ok(());
                        }
                        Err(VerificationError::KeyNotFound) => {
                            log::debug!("Line {}: No match", i + 1);
                            continue;
                        }
                        Err(e) => {
                            log::debug!("Line {}: Stop with {:?} error", i + 1, e);
                            return Err(e);
                        }
                    }
                }
            }
        }
        Err(VerificationError::KeyNotFound)
    }
}

impl HostKeyVerifier for KnownHosts {
    fn verify(&self, name: &str, identity: &Identity) -> VerificationFuture {
        let self_ = self.clone();
        let name: String = name.into();
        let identity = identity.clone();
        Box::pin(async move { self_.verify(&name, &identity).await })
    }
}

impl Default for KnownHosts {
    fn default() -> Self {
        let mut paths = vec![];
        if cfg!(unix) {
            let mut path = PathBuf::new();
            path.push("/etc/ssh/ssh_known_hosts");
            paths.push(path)
        }
        if let Some(mut path) = std::env::home_dir() {
            path.push(".ssh");
            path.push("known_hosts");
            paths.push(path)
        }
        Self { paths }
    }
}

#[derive(Debug)]
struct Line {
    mark: Mark,
    name: Name,
    algo: String,
    pkey: String,
}

#[derive(Debug)]
enum Mark {
    Regular,
    Revoked,
    CertAuthority,
}

#[derive(Debug)]
enum Name {
    Hash(Hash),
    Patterns(Patterns),
}

#[derive(Debug)]
struct Hash {
    alg: usize,
    key: String,
    mac: String,
}

#[derive(Debug)]
struct Patterns(Vec<(bool, Glob)>);

impl Line {
    pub fn parse(input: &str) -> Option<Self> {
        let mut xs = input.split_whitespace();
        let (mark, x) = match xs.next()? {
            "@revoked" => (Mark::Revoked, xs.next()?),
            "@cert-authority" => (Mark::CertAuthority, xs.next()?),
            x => (Mark::Regular, x),
        };
        Some(Self {
            mark,
            name: Name::parse(x)?,
            algo: String::from(xs.next()?),
            pkey: String::from(xs.next()?),
        })
    }

    pub fn verify(self, name: &str, identity: &Identity) -> Result<(), VerificationError> {
        let e = VerificationError::KeyNotFound;
        // Check whether this line is applicable for the host
        assume(self.name.test(name)).ok_or(e)?;
        // Decode the associated public key
        let pubkey = base64::decode(&self.pkey).map_err(|_| e)?;
        let pubkey = PublicKey::decode(&mut BDecoder::from(&pubkey), &self.algo).ok_or(e)?;
        // Check the local public key against the supplied identity (key or certificate)
        match self.mark {
            Mark::Regular if identity.public_key() == pubkey => Ok(()),
            Mark::Revoked if identity.public_key() == pubkey => Err(VerificationError::KeyRevoked),
            Mark::CertAuthority if identity.is_valid_cert(&pubkey) => Ok(()), // FIXME
            _ => Err(VerificationError::KeyNotFound),
        }
    }
}

impl Name {
    fn parse(input: &str) -> Option<Self> {
        if input.starts_with('|') {
            Self::Hash(Hash::parse(input)?).into()
        } else {
            Self::Patterns(Patterns::parse(input)?).into()
        }
    }

    fn test(&self, name: &str) -> bool {
        match &self {
            Self::Hash(hash) => hash.test(name),
            Self::Patterns(patterns) => patterns.test(name),
        }
    }
}

impl Patterns {
    fn parse(input: &str) -> Option<Self> {
        let vec = input
            .split(',')
            .map(|n| {
                if n.starts_with('!') {
                    (true, Glob(String::from(&n[1..])))
                } else {
                    (false, Glob(String::from(n)))
                }
            })
            .collect();
        Some(Self(vec))
    }

    fn test(&self, name: &str) -> bool {
        // Test all globs. Stop immediately on negated match or try all.
        let mut result = false;
        for (negated, glob) in &self.0 {
            if glob.test(name) {
                if *negated {
                    // Return early: This line must not be used for this host
                    return false;
                } else {
                    // DO NOT return early: Negated match might follow!
                    result = true
                }
            }
        }
        result
    }
}

impl Hash {
    const HMAC_SHA1: usize = 1;

    fn parse(input: &str) -> Option<Self> {
        let mut x = input.split('|');
        let _ = x.next();
        Self {
            alg: x.next()?.parse().ok()?,
            key: x.next()?.into(),
            mac: x.next()?.into(),
        }
        .into()
    }

    fn test(&self, name: &str) -> bool {
        match self.alg {
            Self::HMAC_SHA1 => self.test_hmac_sha1(name),
            _ => false,
        }
    }

    fn test_hmac_sha1(&self, name: &str) -> bool {
        use hmac::{Hmac, Mac};
        use sha1::Sha1;

        let f = |self_: &Self| -> Option<()> {
            assume(self_.key.len() == 28).map(drop)?;
            assume(self_.mac.len() == 28).map(drop)?;
            let mut k: [u8; 28] = [0; 28]; // safety margin as base64::decode might panic
            let klen = base64::decode_config_slice(&self_.key, base64::STANDARD, &mut k).ok()?;
            let mut m: [u8; 28] = [0; 28]; // safety margin as base64::decode might panic
            let mlen = base64::decode_config_slice(&self_.mac, base64::STANDARD, &mut m).ok()?;
            let mut hmac = Hmac::<Sha1>::new_varkey(&k[..klen]).ok()?;
            hmac.input(name.as_ref());
            hmac.verify(&m[..mlen]).ok()
        };

        f(self).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_hosts_name_01() {
        let name = Name::parse("*.example.com,10.0.0.?,!bar.example.com").unwrap();

        assert!(name.test("foobar.example.com"));
        assert!(name.test("10.0.0.1"));
        assert!(!name.test("example.com"));
        assert!(!name.test("bar.example.com"));
        assert!(!name.test("10.0.0.11"));
    }

    #[test]
    fn test_known_hosts_name_02() {
        let name =
            Name::parse("|1|F1E1KeoE/eEWhi10WpGv4OdiO6Y=|3988QV0VE8wmZL7suNrYQLITLCg=").unwrap();

        assert!(name.test("192.168.1.61"));
        assert!(!name.test("192.168.1.1"));
    }
}
