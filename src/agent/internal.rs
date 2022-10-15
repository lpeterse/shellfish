use super::AuthAgent;
use super::AuthAgentFuture;
use super::Signature;
use crate::identity::ssh_ed25519::SshEd25519PublicKey;
use crate::identity::Identity;
use crate::util::codec::*;
use ed25519_dalek as ed25519;

#[derive(Debug)]
pub struct InternalAgent {
    identity: Identity,
    keypair: ed25519::Keypair,
}

impl InternalAgent {
    pub fn new_random() -> Self {
        let mut csprng = rand_core::OsRng{};
        let keypair = ed25519::Keypair::generate(&mut csprng);
        let identity = SshCodec::encode(&SshEd25519PublicKey(keypair.public.as_bytes())).unwrap();
        let identity = Identity::from(identity);

        Self { identity, keypair }
    }
}

impl AuthAgent for InternalAgent {
    fn identities(&self) -> AuthAgentFuture<Vec<(Identity, String)>> {
        let identity = self.identity.clone();
        Box::pin(async move { Ok(vec![(identity, String::new())]) })
    }

    fn signature(&self, id: &Identity, data: &[u8], _: u32) -> AuthAgentFuture<Option<Signature>> {
        // FIXME check key
        use ed25519_dalek::Signer;
        let algo = "ssh-ed25519".to_string();
        let blob = self.keypair.sign(data).to_bytes().to_vec();
        let sig = Ok(Some(Signature::new(algo, blob)));
        Box::pin(async move { sig })
    }
}
