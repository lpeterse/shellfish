use super::*;
use crate::auth::*;
use crate::util::glob::*;
use crate::util::*;

use async_std::fs::File;
use async_std::io::{BufReader, Read};
use async_std::prelude::*;
use std::path::PathBuf;
use hmac::crypto_mac::NewMac;

#[derive(Clone, Debug)]
pub struct KnownHostsFiles {
    paths: Vec<PathBuf>,
}

impl KnownHostsFiles {
    /// Loop through all files and lines until either a match or a revocation has been found.
    pub async fn verify(&self, name: &str, identity: &Identity) -> KnownHostsResult {
        // Add wrapping braces if name contains a non-standard port
        let canonical_name;
        let name = if let Some(pos) = name.find(':') {
            let (host, port) = name.split_at(pos);
            if port == ":22" {
                name
            } else {
                canonical_name = format!("[{}]{}", host, port);
                &canonical_name
            }
        } else {
            name
        };
        // Loop through all known_hosts files
        for path in &self.paths {
            log::debug!("Looking for {} in {:?}", name, path);
            match Self::verify_path(&path, name, identity).await? {
                Some(decision) => return Ok(Some(decision)),
                None => continue,
            }
        }
        Err("key not found".into())
    }

    pub async fn verify_path(path: &PathBuf, name: &str, identity: &Identity) -> KnownHostsResult {
        match File::open(path).await {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e)?,
            Ok(file) => Self::verify_file(file, name, identity).await,
        }
    }

    pub async fn verify_file<T: Read + Unpin>(
        file: T,
        name: &str,
        identity: &Identity,
    ) -> KnownHostsResult {
        let mut lines = BufReader::new(file).lines().enumerate();
        while let Some(line) = lines.next().await {
            if let (i, Ok(line)) = line {
                if let Some(line) = Line::parse(&line) {
                    match line.verify(name, identity) {
                        Ok(Some(KnownHostsDecision::Accepted)) => {
                            log::debug!("Line {}: Found matching host key", i + 1);
                            return Ok(Some(KnownHostsDecision::Accepted));
                        }
                        Ok(Some(KnownHostsDecision::Rejected)) => {
                            log::debug!("Line {}: Found revoked host key", i + 1);
                        }
                        Ok(None) => {
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
        Ok(None)
    }
}

impl KnownHosts for KnownHostsFiles {
    fn verify(&self, name: &str, identity: &Identity) -> KnownHostsFuture {
        let self_ = self.clone();
        let name: String = name.into();
        let identity = identity.clone();
        Box::pin(async move { self_.verify(&name, &identity).await })
    }
}

#[allow(deprecated)]
impl Default for KnownHostsFiles {
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

    pub fn verify(self, name: &str, identity: &Identity) -> KnownHostsResult {
        // Check whether this line is applicable for the host
        if self.name.test(name) {
            // Decode the associated public key
            if let Some(pubkey) = base64::decode(&self.pkey).ok() {
                if let Some(pubkey) = decode_public_key(&pubkey)  { // FIXME}, &self.algo) {
                    // Check the local public key against the supplied identity (key or certificate)
                    match self.mark {
                        Mark::Regular if identity.public_key_equals(&pubkey) => {
                            return Ok(Some(KnownHostsDecision::Accepted))
                        }
                        Mark::Revoked if identity.public_key_equals(&pubkey) => {
                            return Ok(Some(KnownHostsDecision::Rejected))
                        }
                        //Mark::CertAuthority if identity.is_valid_cert(&pubkey) => (), // FIXME: support certs
                        _ => (),
                    }
                }
            }
        }
        Ok(None)
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
            hmac.update(name.as_ref());
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
