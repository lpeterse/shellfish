#[derive(Debug)]
pub enum AuthAgentError {
    IoError(std::io::Error),
    FrameError,
    DecoderError,
}

impl From<std::io::Error> for AuthAgentError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        assert_eq!("FrameError", format!("{:?}", AuthAgentError::FrameError));
    }

    #[test]
    fn test_from_io_error() {
        let e = std::io::Error::new(std::io::ErrorKind::Other, "");
        let e: AuthAgentError = e.into();
        match e {
            AuthAgentError::IoError(_) => (),
            _ => panic!()
        }
    }
}
