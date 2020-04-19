use crate::codec::*;

#[derive(Debug, Clone)]
pub enum Exit {
    Status(ExitStatus),
    Signal(ExitSignal),
}

#[derive(Debug, Clone, Copy)]
pub struct ExitStatus(pub u32);

#[derive(Debug, Clone)]
pub struct ExitSignal {
    signal: String,
    core_dumped: bool,
    message: String,
}

impl Encode for ExitStatus {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be(self.0)
    }
}

impl Decode for ExitStatus {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Self(d.take_u32be()?).into()
    }
}

impl Encode for ExitSignal {
    fn size(&self) -> usize {
        Encode::size(&self.signal) + 1 + Encode::size(&self.message) + Encode::size(&"")
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Encode::encode(&self.signal, e);
        e.push_u8(self.core_dumped as u8);
        Encode::encode(&self.message, e);
        Encode::encode(&"", e);
    }
}

impl Decode for ExitSignal {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let signal = Decode::decode(d)?;
        let core_dumped = d.take_bool()?;
        let message = Decode::decode(d)?;
        let _: &str = DecodeRef::decode(d)?;
        Self {
            signal,
            core_dumped,
            message,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_status_debug_01() {
        assert_eq!(format!("{:?}", ExitStatus(23)), "ExitStatus(23)");
    }

    #[test]
    fn test_exit_signal_debug_01() {
        let x = ExitSignal {
            signal: "ABRT".into(),
            core_dumped: true,
            message: "msg".into(),
        };
        assert_eq!(
            format!("{:?}", x),
            "ExitSignal { signal: \"ABRT\", core_dumped: true, message: \"msg\" }"
        );
    }

    #[test]
    fn test_exit_debug_01() {
        assert_eq!(
            format!("{:?}", Exit::Status(ExitStatus(23))),
            "Status(ExitStatus(23))"
        );
    }

    #[test]
    fn test_exit_debug_02() {
        let x = Exit::Signal(ExitSignal {
            signal: "ABRT".into(),
            core_dumped: true,
            message: "msg".into(),
        });
        assert_eq!(
            format!("{:?}", x),
            "Signal(ExitSignal { signal: \"ABRT\", core_dumped: true, message: \"msg\" })"
        );
    }

    #[test]
    fn test_exit_status_encode_01() {
        let x = ExitStatus(23);
        let v = BEncoder::encode(&x);
        assert_eq!(&v[..], &[0, 0, 0, 23][..]);
    }

    #[test]
    fn test_exit_signal_encode_01() {
        let x = ExitSignal {
            signal: "ABRT".into(),
            core_dumped: true,
            message: "msg".into(),
        };
        let v = BEncoder::encode(&x);
        assert_eq!(
            &v[..],
            &[0, 0, 0, 4, 65, 66, 82, 84, 1, 0, 0, 0, 3, 109, 115, 103, 0, 0, 0, 0][..]
        );
    }

    #[test]
    fn test_exit_status_decode_01() {
        let x: ExitStatus = BDecoder::decode(&[0, 0, 0, 23][..]).unwrap();
        assert_eq!(x.0, 23);
    }

    #[test]
    fn test_exit_signal_decode_01() {
        let x: ExitSignal = BDecoder::decode(
            &[
                0, 0, 0, 4, 65, 66, 82, 84, 1, 0, 0, 0, 3, 109, 115, 103, 0, 0, 0, 0,
            ][..],
        )
        .unwrap();
        assert_eq!(x.signal, "ABRT");
        assert_eq!(x.core_dumped, true);
        assert_eq!(x.message, "msg");
    }
}