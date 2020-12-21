use crate::util::codec::*;

use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct DirectTcpIpOpen {
    /// The host where the recipient should connect the channel.
    pub dst_host: String,
    /// The port where the recipient should connect the channel.
    pub dst_port: u16,
    /// The address of the machine from where the connection request originates.
    pub src_addr: IpAddr,
    /// The port from where the connection request originates.
    pub src_port: u16,
}

impl Encode for DirectTcpIpOpen {
    fn size(&self) -> usize {
        let mut n = 0;
        n += 4 + self.dst_host.len();
        n += 4;
        n += 4 + self.src_addr.to_string().len();
        n += 4;
        n
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_str_framed(&self.dst_host)?;
        e.push_u32be(self.dst_port as u32)?;
        e.push_str_framed(&self.src_addr.to_string())?;
        e.push_u32be(self.src_port as u32)
    }
}

impl Decode for DirectTcpIpOpen {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Self {
            dst_host: Decode::decode(d)?,
            dst_port: d.take_u32be()? as u16,
            src_addr: IpAddr::from_str(DecodeRef::decode(d)?).ok()?,
            src_port: d.take_u32be()? as u16,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_encode_01() {
        let msg = DirectTcpIpOpen {
            dst_host: "localhost".into(),
            dst_port: 123,
            src_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
            src_port: 456,
        };
        assert_eq!(
            &[
                0, 0, 0, 9, 108, 111, 99, 97, 108, 104, 111, 115, 116, 0, 0, 0, 123, 0, 0, 0, 9,
                49, 50, 55, 46, 48, 46, 48, 46, 49, 0, 0, 1, 200
            ][..],
            &SliceEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_encode_02() {
        let msg = DirectTcpIpOpen {
            dst_host: "localhost".into(),
            dst_port: 123,
            src_addr: IpAddr::V6(Ipv6Addr::LOCALHOST),
            src_port: 456,
        };
        assert_eq!(
            &[
                0, 0, 0, 9, 108, 111, 99, 97, 108, 104, 111, 115, 116, 0, 0, 0, 123, 0, 0, 0, 3,
                58, 58, 49, 0, 0, 1, 200
            ][..],
            &SliceEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 34] = [
            0, 0, 0, 9, 108, 111, 99, 97, 108, 104, 111, 115, 116, 0, 0, 0, 123, 0, 0, 0, 9, 49,
            50, 55, 46, 48, 46, 48, 46, 49, 0, 0, 1, 200,
        ];
        let actual: DirectTcpIpOpen = SliceDecoder::decode(&buf[..]).unwrap();
        let expected = DirectTcpIpOpen {
            dst_host: "localhost".into(),
            dst_port: 123,
            src_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
            src_port: 456,
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_decode_02() {
        let buf: [u8; 28] = [
            0, 0, 0, 9, 108, 111, 99, 97, 108, 104, 111, 115, 116, 0, 0, 0, 123, 0, 0, 0, 3, 58,
            58, 49, 0, 0, 1, 200,
        ];
        let actual: DirectTcpIpOpen = SliceDecoder::decode(&buf[..]).unwrap();
        let expected = DirectTcpIpOpen {
            dst_host: "localhost".into(),
            dst_port: 123,
            src_addr: IpAddr::V6(Ipv6Addr::LOCALHOST),
            src_port: 456,
        };
        assert_eq!(actual, expected);
    }
}
