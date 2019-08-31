use crate::codec::*;
use crate::codec_ssh::*;
use super::error::*;

use async_std::io::{Read,Write};

#[derive(Clone,Debug,PartialEq)]
pub struct Identification {
    pub version: String,
    pub comment: Option<String>,
}

impl Default for Identification {
    fn default() -> Self {
        Self {
            version: format!("{}_{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            comment: None
        }
    }
}

impl <'a> SshCodec<'a> for Identification {
    fn size(&self) -> usize {
        b"SSH-2.0-".len()
        + self.version.len()
        + match self.comment { None => 0, Some(ref x) => 1 + x.len() }
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        c.push_bytes(b"SSH-2.0-");
        c.push_bytes(&self.version.as_bytes());
        match self.comment { None => (), Some(ref x) => { c.push_u8(' ' as u8); c.push_bytes(&x.as_bytes()); }};
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        c.match_bytes(b"SSH-2.0-")?;
        if c.remaining() > 253 { return None };
        let version = c.take_while(|x| (x as char).is_ascii_graphic() && x != ('-' as u8) && x != (' ' as u8))?;
        if c.is_empty() {
            Some(Self { version: String::from_utf8(version.to_vec()).ok()?, comment: None })
        } else {
            c.take_u8().filter(|x| *x == (' ' as u8))?;
            let comment = c.take_while(|x| (x as char).is_ascii_graphic())?;
            c.take_eoi()?;
            Some(Self { version: String::from_utf8(version.to_vec()).ok()?, comment: Some(String::from_utf8(comment.to_vec()).ok()?) })
        }
    }
}

impl Identification {

    pub const MAX_LEN: usize = 253;

    pub async fn write<T: Write + futures::io::AsyncWrite + Unpin>(self, mut stream: T, buf: &mut [u8]) -> TransportResult<()> {

        SshCodec::encode(&self, &mut Encoder::from(&mut buf[..]));
        let len = SshCodec::size(&self);
        buf[len] = 0x0d;
        buf[len+1] = 0x0a;
        stream.write_all(&buf[..len+2]).await.map_err(TransportError::IoError)?;
        stream.flush().await.map_err(|e| e.into())
    }

    pub async fn read<'a,T: Read + futures::io::AsyncRead + Unpin>(mut stream: T, buf: &'a mut [u8]) -> TransportResult<(Identification, &'a [u8])> {

        fn line_len<'a>(input: &[u8]) -> Option<usize> {
            let mut d = Decoder(input);
            let v = d.take_while(|x| x != 0x0d)?;
            d.take_u8().filter(|x| *x == 0x0d)?;
            d.take_u8().filter(|x| *x == 0x0a)?;
            Some(v.len())
        }

        let mut start: usize = 0;
        let mut end: usize = 0;

        loop {
            end += Read::read(&mut stream, &mut buf[end..]).await?;
            loop {
                match line_len(&buf[start..end]) {
                    None => {
                        break
                    },
                    Some(len) => match SshCodec::decode(&mut Decoder(&buf[start..start + len])) {
                        Some(id) => return Ok((id, &buf[start + len + 2 ..end])),
                        None => start += len + 2
                    }
                }
            }
            if end == buf.len() { break };
        }

        Err(TransportError::InvalidIdentification)
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_ok_01() {
        async_std::task::block_on(async {
            let mut buf = [0 as u8;255];
            let input = "SSH-2.0-version\r\n".as_bytes();
            let rem = "".as_bytes();
            let id = Identification { version: "version".into(), comment: None };

            match Identification::read(input, &mut buf).await {
                Err(e) => panic!("{:?}", e),
                Ok(v) => assert_eq!(v, (id, &rem[..])),
            }
        })
    }

    #[test]
    fn test_read_ok_02() {
        async_std::task::block_on(async {
            let mut buf = [0 as u8;255];
            let input = "SSH-2.0-version comment\r\n".as_bytes();
            let rem = "".as_bytes();
            let id = Identification { version: "version".into(), comment: Some("comment".into()) };

            match Identification::read(input, &mut buf).await {
                Err(e) => panic!("{:?}", e),
                Ok(v) => assert_eq!(v, (id, &rem[..])),
            }
        })
    }

    #[test]
    fn test_read_ok_03() {
        async_std::task::block_on(async {
            let mut buf = [0 as u8;255];
            let input = "\r\n\r\nABC\r\nSSH-2.0-version comment\r\n".as_bytes();
            let rem = "".as_bytes();
            let id = Identification { version: "version".into(), comment: Some("comment".into()) };

            match Identification::read(input, &mut buf).await {
                Err(e) => panic!("{:?}", e),
                Ok(v) => assert_eq!(v, (id, &rem[..])),
            }
        })
    }

    #[test]
    fn test_read_ok_04() {
        async_std::task::block_on(async {
            let mut buf = [0 as u8;255];
            let input = "SSH-2.0-version\r\nKEX".as_bytes();
            let rem = "KEX".as_bytes();
            let id = Identification { version: "version".into(), comment: None };

            match Identification::read(input, &mut buf).await {
                Err(e) => panic!("{:?}", e),
                Ok(v) => assert_eq!(v, (id, &rem[..])),
            }
        })
    }
}
