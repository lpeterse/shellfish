#[derive(Clone, Debug, PartialEq)]
pub struct CertType(pub u32);

impl CertType {
    pub const USER: Self = Self(1);
    pub const HOST: Self = Self(2);
}
