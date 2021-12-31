use shellfish::host::*;
use shellfish::identity::*;
use shellfish::util::BoxFuture;

#[derive(Debug)]
pub struct HostVerifierForTesting {
    delay: std::time::Duration,
    known: Vec<(String, u16, Identity)>,
}

impl HostVerifierForTesting {
    pub fn new(name: &str, port: u16, identity: &Identity) -> Self {
        Self {
            delay: std::time::Duration::from_millis(3),
            known: vec![(name.into(), port, identity.clone())]
        }
    }
}

impl HostVerifier for HostVerifierForTesting {
    fn verify(
        &self,
        name: &str,
        port: u16,
        identity: &Identity,
    ) -> BoxFuture<Result<(), HostVerificationError>> {
        let delay = self.delay;
        let found = self
            .known
            .iter()
            .any(|x| x.0 == name && x.1 == port && &x.2 == identity);
        let result = if found {
            Ok(())
        } else {
            Err(HostVerificationError::Unverifiable)
        };
        Box::pin(async move {
            tokio::time::sleep(delay).await;
            result
        })
    }
}
