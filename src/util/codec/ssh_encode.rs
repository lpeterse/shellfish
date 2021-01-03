use super::*;

/// SSH specific encoding.
pub trait SshEncode {
    #[must_use]
    fn encode<T: SshEncoder>(&self, encoder: &mut T) -> Option<()>;
}

impl SshEncode for () {
    fn encode<T: SshEncoder>(&self, _: &mut T) -> Option<()> {
        Some(())
    }
}

impl SshEncode for String {
    fn encode<T: SshEncoder>(&self, e: &mut T) -> Option<()> {
        e.push_str_framed(&self)
    }
}

impl<A: SshEncode, B: SshEncode> SshEncode for (A, B) {
    fn encode<T: SshEncoder>(&self, e: &mut T) -> Option<()> {
        e.push(&self.0)?;
        e.push(&self.1)
    }
}
