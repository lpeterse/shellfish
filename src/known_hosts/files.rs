mod line;
mod pattern;

use self::line::*;

use super::*;
use crate::auth::*;

use async_std::fs::File;
use async_std::io::{BufReader, Read};
use async_std::prelude::*;
use std::path::PathBuf;

/// A `known_hosts` file processor and verifier.
///
/// The default instance contains the common `known_hosts` file locations.
#[derive(Clone, Debug)]
pub struct KnownHosts {
    paths: Vec<PathBuf>,
}

impl KnownHosts {
    async fn query(&self, name: &str, id: &Identity) -> Result<bool, KnownHostsError> {
        // Add wrapping braces if name contains a non-standard port
        let canonical_name;
        let name = if let Some(pos) = name.find(':') {
            let (host, port) = name.split_at(pos);
            if port == ":22" {
                name
            } else {
                canonical_name = format!("[{}]:{}", host, port);
                &canonical_name
            }
        } else {
            name
        };
        if let Some(cert) = id.as_cert() {
            cert.verify_for_host(name)?;
            self.query_files(name, id, Some(cert.authority())).await
        } else {
            self.query_files(name, id, None).await
        }
    }

    /// Loop through all files and lines until either a match or a revocation has been found.
    async fn query_files(
        &self,
        name: &str,
        id: &Identity,
        ca: Option<&Identity>,
    ) -> Result<bool, KnownHostsError> {
        let mut found = false;
        for path in &self.paths {
            match File::open(path).await {
                Ok(file) => {
                    found |= Self::query_file(name, id, ca, file).await?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    continue;
                }
                Err(e) => Err(e)?,
            }
        }
        Ok(found)
    }

    async fn query_file<T: Read + Unpin>(
        name: &str,
        id: &Identity,
        ca: Option<&Identity>,
        file: T,
    ) -> Result<bool, KnownHostsError> {
        let mut found = false;
        let mut lines = BufReader::new(file).lines();
        while let Some(line) = lines.next().await {
            found |= KnownHostsLine(&line?).test(name, id, ca)?;
        }
        Ok(found)
    }
}

#[allow(deprecated)]
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

impl KnownHostsLike for KnownHosts {
    fn verify(&self, name: &str, id: &Identity) -> BoxFuture<Result<(), KnownHostsError>> {
        let self_ = self.clone();
        let name: String = name.into();
        let id = id.clone();
        Box::pin(async move {
            if self_.query(&name, &id).await? {
                Ok(())
            } else {
                Err(KnownHostsError::Unverifiable)
            }
        })
    }
}
