mod encryption;
mod error;
mod identification;
mod kex;
mod key_streams;
mod packet;

pub use self::encryption::*;
pub use self::identification::*;
pub use self::kex::*;
pub use self::error::*;
pub use self::packet::*;
pub use self::key_streams::*;

use crate::buffer::*;
use crate::keys::*;
use crate::codec::*;
use crate::codec_ssh::*;

use std::convert::{From};
use async_std::io::{Read, Write};
use futures::io::{AsyncRead,AsyncWrite};

pub struct Transport<T> {
    stream: Buffer<T>,
}

impl <T> Transport<T> 
    where
        T: Read + AsyncRead + Unpin,
        T: Write + AsyncWrite + Unpin,
{

    pub async fn new(stream: T) -> TransportResult<Self> {
        let mut buffer = Buffer::new(stream);

        // Send own version string
        let local_id = Identification::default();
        let mut enc = Encoder::from(buffer.alloc(SshCodec::size(&local_id) + 2).await?);
        SshCodec::encode(&local_id, &mut enc);
        enc.push_u8('\r' as u8);
        enc.push_u8('\n' as u8);
        buffer.flush().await?;

        // Drop lines until remote SSH-2.0- version string is recognized
        let remote_id: Identification = loop {
            let line = buffer.read_line(Identification::MAX_LEN).await?;
            match SshCodec::decode(&mut Decoder(line)) {
                None => (),
                Some(id) => break id,
            }
        };

        async fn send<'a, T: Read + AsyncRead + Write + AsyncWrite + Unpin, M: SshCodec<'a>>(buffer: &'a mut Buffer<T>, msg: M) -> TransportResult<()> {
            let packet = Packet::new(msg);
            let packet_size = SshCodec::size(&packet);
            let mut enc = Encoder::from(buffer.alloc(4 + packet_size).await?);
            println!("PPPP {}", 4 + packet_size);
            enc.push_u32be(packet_size as u32);
            SshCodec::encode(&packet, &mut enc);
            Ok(())
        }

        async fn receive<T: Read + AsyncRead + Write + AsyncWrite + Unpin>(buffer: &mut Buffer<T>) -> TransportResult<&[u8]> {
            let packet_size = Decoder(buffer.read_exact(4).await?).take_u32be().unwrap() as usize;
            let packet = buffer.read_exact(packet_size).await?;
            Ok(&packet[1..])
        }

        let mut kex = Kex::new_client(local_id, remote_id);
        if let Some(kex_init) = kex.init()? {
            send(&mut buffer, kex_init).await?;
            buffer.flush().await?;
        }

        loop {
            buffer.fetch(8).await?; // Don't decode size before 8 bytes have arrived
            let packet = receive(&mut buffer).await?;
            if let Some(x) = SshCodec::decode(&mut Decoder(packet)) {
                if let Some(kex_ecdh_init) = kex.push_kex_init(x)? {
                    send(&mut buffer, kex_ecdh_init).await?;
                    buffer.flush().await?;
                }
                continue;
            }
            if let Some(x) = SshCodec::decode(&mut Decoder(packet)) {
                let output = kex.push_kex_ecdh_reply(x)?;
                println!("{:?}", output);
                break;
            }
        }

        Ok(Self {
            stream: buffer,
        })
    }

    pub async fn read_message<'a>(&'a mut self) -> TransportResult<Message<&'a [u8]>> {
        panic!("")
    }

    pub async fn write_message<X: AsRef<[u8]>>(&mut self, msg: Message<X>) -> TransportResult<()> {
        panic!("")
    }
}

pub struct Message<T: AsRef<[u8]>> (pub T);

