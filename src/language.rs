#[derive(Debug,Clone)]
pub struct Language (Vec<u8>);

impl AsRef<[u8]> for Language {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<&[u8]> for Language {
    fn from(x: &[u8]) -> Self {
        Self(Vec::from(x))
    }
}