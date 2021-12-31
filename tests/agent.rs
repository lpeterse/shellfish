use ed25519_dalek as ed25519;
use shellfish::agent::*;
use shellfish::identity::ssh_ed25519::SshEd25519PublicKey;
use shellfish::identity::*;
use shellfish::util::codec::*;

#[derive(Debug)]
pub struct AuthAgentForTesting {
    delay: std::time::Duration,
    identities: Vec<(Identity, String, ed25519::Keypair)>,
}

impl AuthAgentForTesting {
    pub fn new() -> Self {
        let secret_key_bytes: [u8; ed25519::SECRET_KEY_LENGTH] = [
            157, 097, 177, 157, 239, 253, 090, 096, 186, 132, 074, 244, 146, 236, 044, 196, 068,
            073, 197, 105, 123, 050, 105, 025, 112, 059, 172, 003, 028, 174, 127, 096,
        ];
        let secret = ed25519::SecretKey::from_bytes(&secret_key_bytes).unwrap();
        let public = ed25519::PublicKey::from(&secret);
        let identity = SshCodec::encode(&SshEd25519PublicKey(public.as_bytes())).unwrap();
        let identity = Identity::from(identity);
        let comment = "KEY 1 (ed25519)".to_string();
        let keypair = ed25519::Keypair { public, secret };

        Self {
            delay: std::time::Duration::from_millis(7),
            identities: vec![(identity, comment, keypair)],
        }
    }
}

impl AuthAgent for AuthAgentForTesting {
    fn identities(&self) -> AuthAgentFuture<Vec<(Identity, String)>> {
        let delay = self.delay;
        let ids: Vec<_> = self
            .identities
            .iter()
            .map(|(i, c, _)| (i.clone(), c.clone()))
            .collect();
        Box::pin(async move {
            tokio::time::sleep(delay).await;
            Ok(ids)
        })
    }

    fn signature(&self, id: &Identity, data: &[u8], _: u32) -> AuthAgentFuture<Option<Signature>> {
        let delay = self.delay;
        let sig = if let Some(x) = self.identities.iter().find(|x| &x.0 == id) {
            use ed25519_dalek::Signer;
            let algo = "ssh-ed25519".to_string();
            let blob = x.2.sign(data).to_bytes().to_vec();
            Some(Signature::new(algo, blob))
        } else {
            None
        };

        Box::pin(async move {
            tokio::time::sleep(delay).await;
            Ok(sig)
        })
    }
}
