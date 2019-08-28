mod message;

pub use self::message::*;
use crate::codec::*;
use crate::codec_ssh::*;

use std::char;
use async_std::task::{spawn, JoinHandle, sleep};
use async_std::io::{Read, Write};

pub struct Transport {
    task: JoinHandle<()>
}

impl Transport{
    pub async fn new<T: Read + Write>(stream: T) -> Self {
        let task = spawn(async move {
            sleep(std::time::Duration::from_secs(2)).await;
            println!("HUHU");
        });

        Self {
            task
        }
    }
}

#[derive(Clone)]
pub struct Identification {
    version: Vec<u8>,
    comment: Option<Vec<u8>>,
}

impl <'a> SshCodec<'a> for Identification {
    fn size(&self) -> usize {
        b"SSH-2.0-".len()
        + self.version.len()
        + match self.comment { None => 0, Some(ref x) => 1 + x.len() }
        + 2 
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        c.push_bytes(b"SSH-2.0-");
        c.push_bytes(&self.version);
        match self.comment { None => (), Some(ref x) => { c.push_u8(0x20); c.push_bytes(&x); }};
        c.push_u8(0x0d);
        c.push_u8(0x0a);
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        c.match_bytes(b"SSH-2.0-")?;
        let version = c.take_while(|x| (x as char).is_ascii_graphic() && x != ('-' as u8) && x != 0x13)?;
        match c.take_u8() {
            Some(0x13) => {
                c.take_u8().filter(|x| *x == 0x10)?;
                Some(Self { version: version.into(), comment: None })
            },
            Some(0x20) => {
                let comment = c.take_while(|x| (x as char).is_ascii_graphic() && x != ('-' as u8) && x != 0x13)?;
                c.take_u8().filter(|x| *x == 0x10)?;
                Some(Self { version: version.into(), comment: Some(comment.into()) })
            },
            _ => None,
        }
    }
}

async fn read_version_string<T: Read + futures::io::AsyncRead + Unpin>(mut stream: T) -> async_std::io::Result<(Identification, Vec<u8>)> {

    let mut buf = [0;255];

    loop {
        let len = Read::read(&mut stream, &mut buf).await?;
        fn take_line<'a>(input: &'a [u8]) -> Option<&'a [u8]> {
            let mut d = Decoder(input);
            let v = d.take_while(|x| x != 0x13)?;
            d.take_u8().filter(|x| *x == 0x13)?;
            d.take_u8().filter(|x| *x == 0x10)?;
            Some(v)
        }
        //take_line(len);
        panic!("")
    }
}


pub struct RingBuffer<T> {
    off: usize,
    len: usize,
    buf: T
}

impl <T> RingBuffer<T> {
    pub fn shrink(&mut self, len: usize) {

    }
    pub fn extend(&mut self, len: usize) {

    }
}
