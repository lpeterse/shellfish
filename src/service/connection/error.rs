use super::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ConnectionError {
    Terminated,
    IoError(std::io::ErrorKind),
    TransportError(TransportError),
    ChannelOpenFailure(ChannelOpenFailure),
    ChannelIdInvalid,
    ChannelRequestFailure,
    ChannelFailureUnexpected,
    ChannelSuccessUnexpected,
    ChannelWindowSizeUnderrun,
    RequestSenderDropped,
    RequestReceiverDropped,
    RequestUnexpectedResponse,
}

impl From<std::io::Error> for ConnectionError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.kind())
    }
}

impl From<TransportError> for ConnectionError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}

impl From<ChannelOpenFailure> for ConnectionError {
    fn from(e: ChannelOpenFailure) -> Self {
        Self::ChannelOpenFailure(e)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_debug_01() {
        assert_eq!("Terminated", format!("{:?}", ConnectionError::Terminated));
        assert_eq!(
            "IoError(Other)",
            format!("{:?}", ConnectionError::IoError(std::io::ErrorKind::Other))
        );
        assert_eq!(
            "TransportError(BadPacketLength)",
            format!(
                "{:?}",
                ConnectionError::TransportError(TransportError::BadPacketLength)
            )
        );
        assert_eq!(
            "ChannelOpenFailure(ChannelOpenFailure { reason: Reason::ADMINISTRATIVELY_PROHIBITED })",
            format!(
                "{:?}",
                ConnectionError::ChannelOpenFailure(ChannelOpenFailure {
                    reason: Reason::ADMINISTRATIVELY_PROHIBITED
                })
            )
        );
        assert_eq!(
            "ChannelIdInvalid",
            format!("{:?}", ConnectionError::ChannelIdInvalid)
        );
        assert_eq!(
            "ChannelRequestFailure",
            format!("{:?}", ConnectionError::ChannelRequestFailure)
        );
        assert_eq!(
            "ChannelFailureUnexpected",
            format!("{:?}", ConnectionError::ChannelFailureUnexpected)
        );
        assert_eq!(
            "ChannelSuccessUnexpected",
            format!("{:?}", ConnectionError::ChannelSuccessUnexpected)
        );
        assert_eq!(
            "ChannelWindowSizeUnderrun",
            format!("{:?}", ConnectionError::ChannelWindowSizeUnderrun)
        );
        assert_eq!(
            "RequestSenderDropped",
            format!("{:?}", ConnectionError::RequestSenderDropped)
        );
        assert_eq!(
            "RequestReceiverDropped",
            format!("{:?}", ConnectionError::RequestReceiverDropped)
        );
        assert_eq!(
            "RequestUnexpectedResponse",
            format!("{:?}", ConnectionError::RequestUnexpectedResponse)
        );
    }

    #[test]
    fn test_from_io_error_01() {
        match std::io::Error::new(std::io::ErrorKind::Other, "").into() {
            ConnectionError::IoError(_) => (),
            _ => panic!(""),
        }
    }

    #[test]
    fn test_from_transport_error_01() {
        match TransportError::BadPacketLength.into() {
            ConnectionError::TransportError(_) => (),
            _ => panic!(""),
        }
    }

    #[test]
    fn test_from_channel_open_failure_01() {
        let x = ChannelOpenFailure {
            reason: Reason::ADMINISTRATIVELY_PROHIBITED,
        };
        match x.into() {
            ConnectionError::ChannelOpenFailure(_) => (),
            _ => panic!(""),
        }
    }
}
