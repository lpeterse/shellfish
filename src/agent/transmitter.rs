use super::*;
use crate::util::*;
use crate::transport::Socket;

use async_std::io::prelude::WriteExt;
use async_std::os::unix::net::UnixStream;

pub struct Transmitter<S: Socket = UnixStream> {
    socket: S,
}

impl <S: Socket> Transmitter<S> {
    const MAX_FRAME_LEN: usize = 35000;

    pub async fn send<Msg: Encode>(&mut self, msg: &Msg) -> Result<(), AuthAgentError> {
        let vec = BEncoder::encode(&Frame::new(msg));
        self.socket.write_all(&vec).await?;
        Ok(())
    }

    pub async fn receive<Msg: Decode>(&mut self) -> Result<Msg, AuthAgentError> {
        let mut len: [u8; 4] = [0; 4];
        self.socket.read_exact(&mut len[..]).await?;
        let len = u32::from_be_bytes(len) as usize;
        assume(len <= Self::MAX_FRAME_LEN).ok_or(AuthAgentError::FrameError)?;
        let mut vec = Vec::with_capacity(len);
        vec.resize(len, 0);
        self.socket.read_exact(&mut vec[..]).await?;
        BDecoder::decode(&vec).ok_or(AuthAgentError::DecoderError)
    }
}

impl <S: Socket> From<S> for Transmitter<S> {
    fn from(socket: S) -> Self {
        Self { socket }
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    use async_std::io::ReadExt;
    use async_std::os::unix::net::UnixListener;

    fn random_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        path.push(format!("{}.socket", t));
        path
    }

    /// Tests opening the domain socket (happy path)
    #[test]
    fn test_new_01() {
        let path = random_path();
        let path_ = path.clone();
        let x: Result<(), AuthAgentError> = futures::executor::block_on(async move {
            let l = UnixListener::bind(&path_).await?;
            let _ = Transmitter::new(&path_).await?;
            let _ = l; // keep l alive
            Ok(())
        });
        let _ = std::fs::remove_file(path);
        x.unwrap()
    }


    /// Tests opening the domain socket (connection refused)
    #[test]
    fn test_new_02() {
        let path = random_path();
        let x: Result<(), AuthAgentError> = futures::executor::block_on(async move {
            let _ = Transmitter::new(&path).await?;
            Ok(())
        });
        match x {
            Err(AuthAgentError::IoError(_)) => (),
            Err(e) => panic!(e),
            _ => panic!("shall not have succeeded"),
        }
    }

    // Tests sending a frame
    #[test]
    fn test_send_01() {
        let path = random_path();
        let path_ = path.clone();
        let x: Result<(), AuthAgentError> = futures::executor::block_on(async move {
            let l = UnixListener::bind(&path_).await?;
            let mut t = Transmitter::new(&path_).await?;
            let (mut s, _) = l.accept().await?;
            t.send(&String::from("data")).await?;
            let expected: [u8; 12] = [0, 0, 0, 8, 0, 0, 0, 4, 100, 97, 116, 97];
            let mut actual: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            s.read(&mut actual).await?;
            assert_eq!(actual, expected);
            Ok(())
        });
        let _ = std::fs::remove_file(path);
        x.unwrap()
    }

    // Tests receiving a frame
    #[test]
    fn test_receive_01() {
        let path = random_path();
        let path_ = path.clone();
        let x: Result<(), AuthAgentError> = futures::executor::block_on(async move {
            let l = UnixListener::bind(&path_).await?;
            let mut t = Transmitter::new(&path_).await?;
            let data: [u8; 12] = [0, 0, 0, 8, 0, 0, 0, 4, 100, 97, 116, 97];
            let (mut s, _) = l.accept().await?;
            s.write_all(&data).await?;
            let msg: String = t.receive().await?;
            let _ = s; // keep s alive
            assert_eq!(msg, "data");
            Ok(())
        });
        let _ = std::fs::remove_file(path);
        x.unwrap()
    }

    // Tests for `FrameError` when receiving a frame that is too large
    #[test]
    fn test_receive_02() {
        let path = random_path();
        let path_ = path.clone();
        let x: Result<(), AuthAgentError> = futures::executor::block_on(async move {
            let l = UnixListener::bind(&path_).await?;
            let mut t = Transmitter::new(&path_).await?;
            let data: [u8; 4] = [0, 0, 255, 255];
            let (mut s, _) = l.accept().await?;
            s.write_all(&data).await?;
            let msg: String = t.receive().await?;
            let _ = s; // keep s alive
            assert_eq!(msg, "data");
            Ok(())
        });
        let _ = std::fs::remove_file(path);
        match x {
            Err(AuthAgentError::FrameError) => (),
            Err(e) => panic!(e),
            _ => panic!("shall not have succeeded"),
        }
    }

    // Tests for `DecoderError` when receiving an invalid frame
    #[test]
    fn test_receive_03() {
        let path = random_path();
        let path_ = path.clone();
        let x: Result<(), AuthAgentError> = futures::executor::block_on(async move {
            let l = UnixListener::bind(&path_).await?;
            let mut t = Transmitter::new(&path_).await?;
            let data: [u8; 8] = [0, 0, 0, 4, 0, 0, 0, 23];
            let (mut s, _) = l.accept().await?;
            s.write_all(&data).await?;
            let _: String = t.receive().await?;
            let _ = s; // keep s alive
            Ok(())
        });
        let _ = std::fs::remove_file(path);
        match x {
            Err(AuthAgentError::DecoderError) => (),
            Err(e) => panic!(e),
            _ => panic!("shall not have succeeded"),
        }
    }
}
*/
