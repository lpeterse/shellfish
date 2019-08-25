#[derive(Clone, Debug)]
pub struct UnknownPublicKey {
    pub algo: String,
    pub key: Vec<u8>,
}