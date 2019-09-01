mod error;
mod identification;
mod message;
mod packet;

pub use self::message::*;
pub use self::identification::*;
pub use self::error::*;
pub use self::packet::*;

use crate::buffer::*;
use crate::keys::*;
use crate::codec::*;
use crate::codec_ssh::*;

use std::convert::{From};
use async_std::task::{spawn, JoinHandle, sleep};
use async_std::io::{Read, Write};
use futures::stream::Stream;
use futures::io::{AsyncRead,AsyncWrite};

pub struct Transport<T> {
    stream: Buffer<T>,
    buf: [u8;32000],
}

impl <T> Transport<T> 
    where
        T: Read + futures::io::AsyncRead + Unpin,
        T: Write + futures::io::AsyncWrite + Unpin,
{

    pub async fn new(mut stream: T) -> TransportResult<Self> {
        let mut buffer = Buffer::new(stream);

        // Send own version string
        let id = Identification::default();
        let mut enc = Encoder::from(buffer.alloc(SshCodec::size(&id) + 2).await?);
        SshCodec::encode(&id, &mut enc);
        enc.push_u8('\r' as u8);
        enc.push_u8('\n' as u8);
        buffer.flush().await?;

        // Drop lines until remote SSH-2.0- version string is recognized
        let id: Identification = loop {
            let line = buffer.read_line(Identification::MAX_LEN).await?;
            match SshCodec::decode(&mut Decoder(line)) {
                None => (),
                Some(id) => break id,
            }
        };

        // Read packet size
        //buffer.fetch(8).await?; // Don't decode size before 8 bytes have arrived
        let packet_size = Decoder(buffer.read_exact(4).await?).take_u32be().unwrap() as usize;
        println!("SIZE {}", packet_size);
        let packet = &mut buffer.read_exact(packet_size).await?[1..];
        println!("{:?}, {:?}, {:?}", id, Identification::default(), packet);
        let packet: KexInit = SshCodec::decode(&mut Decoder(packet)).unwrap();
        println!("{:?}, {:?}, {:?}", id, Identification::default(), packet);

        // Send KexInit packet
        let packet = Packet::new(KexInit::new(KexCookie::new()));
        let packet_size = SshCodec::size(&packet);
        let mut enc = Encoder::from(buffer.alloc(4 + packet_size).await?);
        println!("PPPP {}", 4 + packet_size);
        enc.push_u32be(packet_size as u32);
        SshCodec::encode(&packet, &mut enc);
        buffer.flush().await?;

        // Send KexEcdhInit packet
        let packet = Packet::new(KexEcdhInit::new(Ed25519PublicKey::new()));
        let packet_size = SshCodec::size(&packet);
        let mut enc = Encoder::from(buffer.alloc(4 + packet_size).await?);
        enc.push_u32be(packet_size as u32);
        SshCodec::encode(&packet, &mut enc);
        buffer.flush().await?;

        // Read next packet
        let packet_size = Decoder(buffer.read_exact(4).await?).take_u32be().unwrap() as usize;
        println!("SIZE {}", packet_size);
        let packet = &mut buffer.read_exact(packet_size).await?[1..];
        println!("{:?}", packet);
        let packet: KexEcdhReply = SshCodec::decode(&mut Decoder(packet)).unwrap();
        println!("{:?}", packet);

        //let a = buffer.read(255).await?;
        //let mut buf = [0;255];
        //SshCodec::encode(&)
        //Identification::default().write(&mut stream, &mut buffer).await?;
        //let (id, leftover) = Identification::read(&mut stream, &mut buf).await?;


        //let mut buf_read = Vec::from(leftover);
        //let mut buf_write = Vec::with_capacity(32000);

        //let cookie = initial_kex().await?;

        Ok(Self {
            stream: buffer,
            buf: [0;32000],
        })
    }

    pub async fn read_message<'a>(&'a mut self) -> TransportResult<Message<&'a [u8]>> {
        Ok(Message(&self.buf[23..]))
    }

    pub async fn write_message<X: AsRef<[u8]>>(&mut self, msg: Message<X>) -> TransportResult<()> {
        panic!("")
    }
}

pub struct Message<T: AsRef<[u8]>> (pub T);

