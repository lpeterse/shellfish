#[derive(Debug)]
pub enum AgentError {
    IoError(std::io::Error),
    FrameError,
    DecoderError,
}

impl From<std::io::Error> for AgentError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        assert_eq!("FrameError", format!("{:?}", AgentError::FrameError));
    }

    #[test]
    fn test_from_io_error() {
        let e = std::io::Error::new(std::io::ErrorKind::Other, "");
        let e: AgentError = e.into();
        match e {
            AgentError::IoError(_) => (),
            _ => panic!()
        }
    }
}
