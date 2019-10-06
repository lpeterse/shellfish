use std::convert::TryFrom;

#[derive(Debug,Clone,PartialEq)]
pub enum CompressionAlgorithm {
    None,
    ZlibAtOpenSshDotCom,
    Unknown(String)
}

impl CompressionAlgorithm {
    pub fn supported() -> Vec<Self> {
        vec![Self::None]
    }
}

impl AsRef<[u8]> for CompressionAlgorithm {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::None => b"none",
            Self::ZlibAtOpenSshDotCom => b"zlib@openssh.com",
            Self::Unknown(s) => s.as_bytes(),
        }
    }
}

impl TryFrom<&[u8]> for CompressionAlgorithm {

    type Error = std::string::FromUtf8Error;

    fn try_from(x: &[u8]) -> Result<Self, std::string::FromUtf8Error> {
        Ok(
            if x == Self::None.as_ref() {
                Self::None
            } 
            else if x == Self::ZlibAtOpenSshDotCom.as_ref() {
                Self::ZlibAtOpenSshDotCom
            }
            else {
                Self::Unknown(String::from_utf8(Vec::from(x))?)
            }
        )
    }
}
