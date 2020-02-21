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
mod test {
    use super::*;

    #[test]
    fn test_debug_01() {
        assert_eq!("FrameError", format!("{:?}", AgentError::FrameError));
    }
}
