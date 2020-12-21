use super::*;

/// SSH specific decoding (inverse of `Encode`).
pub trait Decode: Sized {
    #[must_use]
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self>;
}

/// SSH specific decode that allows the result to contain references into the input.
///
/// This is useful to avoid unnecessary intermediate allocations in cases where
/// the result is short-lived and can be processed while the input is still in scope.
pub trait DecodeRef<'a>: Sized {
    #[must_use]
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self>;
}

/// `Decode  is a stronger property than `DecodeRef` so everything that is `Decode` can
/// automatically inherit `DecodeRef`.
impl<'a, T: Decode> DecodeRef<'a> for T {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Decode::decode(d)
    }
}

impl Decode for () {
    #[inline]
    fn decode<'a, D: Decoder<'a>>(_: &mut D) -> Option<Self> {
        Some(())
    }
}

impl Decode for u32 {
    #[inline]
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u32be()
    }
}

impl Decode for u64 {
    #[inline]
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u64be()
    }
}

impl Decode for String {
    #[inline]
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        DecodeRef::decode(d).map(|x: &str| String::from(x))
    }
}

impl<'a> DecodeRef<'a> for &'a str {
    #[inline]
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        c.take_str(len as usize)
    }
}

impl<'a> DecodeRef<'a> for &'a [u8] {
    #[inline]
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        c.take_bytes(len as usize)
    }
}

impl<T, Q> Decode for (T, Q)
where
    T: Decode,
    Q: Decode,
{
    #[inline]
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let t = Decode::decode(c)?;
        let q = Decode::decode(c)?;
        Some((t, q))
    }
}

impl<A: Decode, B: Decode> Decode for Result<A, B> {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        // The decoder state needs to be cloned as the input needs to be restored on failure.
        None.or_else(|| {
            let mut d_ = d.clone();
            let r = DecodeRef::decode(&mut d_).map(Ok);
            if r.is_some() {
                *d = d_
            };
            r
        })
        .or_else(|| {
            let mut d_ = d.clone();
            let r = DecodeRef::decode(&mut d_).map(Err);
            if r.is_some() {
                *d = d_
            };
            r
        })
    }
}

impl<T: Decode> Decode for Vec<T> {
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        let len = c.take_u32be()?;
        // NB: Don't use `with_capacity` here as it might
        // lead to remote triggered resource exhaustion.
        let mut v = Vec::new();
        for _ in 0..len {
            v.push(DecodeRef::decode(c)?);
        }
        Some(v)
    }
}
