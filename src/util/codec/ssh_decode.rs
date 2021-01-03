use super::*;

/// SSH specific decoding.
pub trait SshDecode: Sized {
    #[must_use]
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self>;
}

/// SSH specific decoding that allows the result to hold references into the input.
///
/// This is useful to avoid unnecessary intermediate allocations in cases where
/// the result is short-lived and can be processed while the input is still in scope.
pub trait SshDecodeRef<'a>: Sized {
    #[must_use]
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self>;
}

/// `Decode  is a stronger property than `SshDecodeRef` so everything that is `Decode` can
/// automatically inherit `SshDecodeRef`.
impl<'a, T: SshDecode> SshDecodeRef<'a> for T {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        SshDecode::decode(d)
    }
}

impl SshDecode for () {
    fn decode<'a, D: SshDecoder<'a>>(_: &mut D) -> Option<Self> {
        Some(())
    }
}

impl SshDecode for String {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_str_framed().map(String::from)
    }
}

impl<T: SshDecode, Q: SshDecode> SshDecode for (T, Q) {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        Some((d.take()?, d.take()?))
    }
}

impl<A: SshDecode, B: SshDecode> SshDecode for Result<A, B> {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        let mut dd = d.clone();
        if let Some(x) = d.take() {
            Some(Ok(x))
        } else if let Some(x) = dd.take() {
            *d = dd;
            Some(Err(x))
        } else {
            None
        }
    }
}
