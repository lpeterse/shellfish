mod line;
mod pattern;

use self::line::*;
use super::*;
use crate::util::runtime::AsyncRead;
use crate::util::runtime::File;
use futures_util::StreamExt;
use std::path::PathBuf;

/// A `known_hosts` file processor and verifier.
///
/// The default instance contains the common `known_hosts` file locations.
#[derive(Clone, Debug)]
pub struct KnownHosts {
    paths: Vec<PathBuf>,
}

impl KnownHosts {
    async fn query(
        &self,
        name: &str,
        port: u16,
        id: &Identity,
    ) -> Result<bool, HostVerificationError> {
        let host;
        let host = match port {
            22 => name,
            _ => {
                host = format!("[{}]:{}", name, port);
                &host
            }
        };
        if let Some(cert) = id.as_cert() {
            cert.verify_for_host(host)?;
            self.query_files(host, id, Some(cert.authority())).await
        } else {
            self.query_files(host, id, None).await
        }
    }

    /// Loop through all files and lines until either a match or a revocation has been found.
    async fn query_files(
        &self,
        host: &str,
        id: &Identity,
        ca: Option<&Identity>,
    ) -> Result<bool, HostVerificationError> {
        let mut found = false;
        for path in &self.paths {
            match File::open(path).await {
                Ok(file) => {
                    found |= Self::query_file(host, id, ca, file).await?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    continue;
                }
                Err(e) => Err(e)?,
            }
        }
        Ok(found)
    }

    async fn query_file<T: AsyncRead + Unpin>(
        host: &str,
        id: &Identity,
        ca: Option<&Identity>,
        file: T,
    ) -> Result<bool, HostVerificationError> {
        let mut found = false;
        match () {
            #[cfg(feature = "rt-tokio")]
            () => {
                use tokio::io::AsyncBufReadExt;
                let mut lines = tokio::io::BufReader::new(file).lines();
                while let Some(line) = lines.next_line().await? {
                    found |= KnownHostsLine(&line).test(host, id, ca)?;
                }
            }
            #[cfg(feature = "rt-async")]
            () => {
                use futures_util::io::AsyncBufReadExt;
                let mut lines = futures_util::io::BufReader::new(file).lines();
                while let Some(line) = lines.next().await {
                    found |= KnownHostsLine(&line?).test(host, id, ca)?;
                }
            }
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

impl HostVerifier for KnownHosts {
    fn verify(
        &self,
        name: &str,
        port: u16,
        id: &Identity,
    ) -> BoxFuture<Result<(), HostVerificationError>> {
        let self_ = self.clone();
        let name: String = name.into();
        let id = id.clone();
        Box::pin(async move {
            if self_.query(&name, port, &id).await? {
                Ok(())
            } else {
                Err(HostVerificationError::Unverifiable)
            }
        })
    }
}
