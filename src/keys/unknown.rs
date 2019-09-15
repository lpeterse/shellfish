#[derive(Clone, Debug, PartialEq)]
pub struct UnknownPublicKey {
    pub algo: String,
    pub key: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct UnknownSignature {
    pub algo: String,
    pub signature: Vec<u8>,
}
