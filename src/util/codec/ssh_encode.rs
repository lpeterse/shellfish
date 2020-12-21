use super::*;

/// SSH specific encoding.
pub trait Encode {
    fn size(&self) -> usize;
    #[must_use]
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()>;
}

impl Encode for () {
    #[inline]
    fn size(&self) -> usize {
        0
    }
    #[inline]
    fn encode<E: SshEncoder>(&self, _: &mut E) -> Option<()> {
        Some(())
    }
}

impl Encode for String {
    #[inline]
    fn size(&self) -> usize {
        4 + self.len()
    }
    #[inline]
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_str_framed(&self)
    }
}

impl<T: Encode, Q: Encode> Encode for (T, Q) {
    #[inline]
    fn size(&self) -> usize {
        self.0.size() + self.1.size()
    }
    #[inline]
    fn encode<E: SshEncoder>(&self, c: &mut E) -> Option<()> {
        self.0.encode(c)?;
        self.1.encode(c)
    }
}

/// A vector is encoded by encoding its number of elements as u32 and each element according to its
/// own encoding rules.
impl<T: Encode> Encode for Vec<T> {
    fn size(&self) -> usize {
        4 + self.iter().map(Encode::size).sum::<usize>()
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u32be(self.len() as u32)?;
        for x in self {
            Encode::encode(x, e)?;
        }
        Some(())
    }
}
