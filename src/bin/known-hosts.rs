use base64;
use rssh::algorithm::authentication::*;
use rssh::codec::*;
use rssh::util::*;
use rssh::glob::*;

#[derive(Clone, Debug)]
pub struct HostLine {
    marker: Option<Marker>,
    names: Vec<Pattern>,
    algorithm: String,
    key: String,
}

#[derive(Clone, Debug)]
pub struct Pattern {
    negated: bool,
    glob: Glob,
}

#[derive(Clone, Debug)]
pub enum Marker {
    Revoked,
    CertAuthority,
}

impl HostLine {
    pub fn parse(x: String) -> Option<Self> {
        let mut x = x.split_whitespace();
        let (marker, x2) = match x.next()? {
            "@revoked" => (Some(Marker::Revoked), x.next()?),
            "@cert-authority" => (Some(Marker::CertAuthority), x.next()?),
            x1 => (None, x1),
        };
        let names = x2
            .split(',')
            .map(|n| {
                if n.starts_with('!') {
                    Pattern {
                        negated: true,
                        glob: Glob(String::from(&n[1..])),
                    }
                } else {
                    Pattern {
                        negated: false,
                        glob: Glob(String::from(n)),
                    }
                }
            })
            .collect();
        let algorithm = String::from(x.next()?);
        let key = String::from(x.next()?);
        Some(Self {
            marker,
            names: names,
            algorithm,
            key,
        })
    }

    pub fn verify(self, name: &str, identity: &HostIdentity) -> Option<bool> {
        let mut name_matches_pattern = false;
        for pattern in &self.names {
            if pattern.glob.test(name) {
                if pattern.negated {
                    // Return early: This line must not be used for this host
                    return None;
                } else {
                    // DO NOT break early: Negated pattern might follow!
                    name_matches_pattern = true;
                }
            }
        }
        // Check whether this line is applicable for the host
        assume(name_matches_pattern)?;
        // Decode the associated public key
        let trusted_key = base64::decode(&self.key).ok()?;
        let trusted_key: HostIdentity = BDecoder::decode(trusted_key.as_ref())?;
        assume(self.algorithm == trusted_key.algorithm())?;
        // Check the local public key against the supplied identity (key or certificate)
        match self.marker {
            Some(Marker::CertAuthority) if identity.is_valid_cert(&trusted_key) => Some(true),
            Some(Marker::Revoked) if identity.is_pubkey(&trusted_key) => Some(false),
            None if identity.is_pubkey(&trusted_key) => Some(true),
            _ => None,
        }
    }
}

async fn verify(name: &str, identity: &HostIdentity) -> bool {
    use async_std::fs::File;
    use async_std::io::BufReader;
    use async_std::prelude::*;

    let paths: Vec<std::path::PathBuf> = dirs::home_dir()
        .map(|mut p| {
            p.push(".ssh/known_hosts");
            p
        })
        .into_iter()
        .collect();

    // Loop through all files and lines until either a positive or negative
    // answer has been found and then return early.
    for path in paths {
        log::debug!("Trying {:?}", path);
        match File::open(path.clone()).await {
            Ok(file) => {
                let mut lines = BufReader::new(file).lines();
                while let Some(line) = lines.next().await {
                    if let Ok(line) = line {
                        if let Some(line) = HostLine::parse(line) {
                            if let Some(x) = line.verify(name, identity) {
                                return x;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("Can't open {:?}: {}", path, e);
            }
        }
    }

    false
}

fn main() {
    env_logger::init();
    let id = HostIdentity::Ed25519Key(SshEd25519PublicKey([0; 32]));
    let _ = futures::executor::block_on(verify("[localhost]:2200", &id));
}
