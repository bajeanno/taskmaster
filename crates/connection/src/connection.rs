use std::{io::Cursor, marker::PhantomData};

use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter};

use crate::{Error, FrameDecodeError, Result};

/// Struct used to send and recieve any type that implements serdes `Serialize` and
/// `DeserializeOwned`.
///
/// Example:
/// ```
/// use connection::Connection;
///
/// tokio::runtime::Runtime::new().unwrap().block_on(async {
///     let (client, server) = tokio::io::duplex(1024);
///     let mut client = Connection::<_, String, i32>::new(client, 1024);
///     let mut server = Connection::<_, i32, String>::new(server, 1024);
///
///     client.write_frame(&42).await.unwrap();
///     server.write_frame(&"Is the answer".to_string()).await.unwrap();
///
///     assert_eq!(42, server.read_frame().await.unwrap().unwrap());
///     assert_eq!("Is the answer".to_string(), client.read_frame().await.unwrap().unwrap());
/// });
/// ```
#[derive(Debug)]
pub struct Connection<Socket, InputFrame, OutputFrame> {
    stream: BufWriter<Socket>,
    buffer: Vec<u8>,

    _input_frame_type: PhantomData<InputFrame>,
    _output_frame_type: PhantomData<OutputFrame>,
}

impl<Socket, InputFrame, OutputFrame> Connection<Socket, InputFrame, OutputFrame>
where
    Socket: AsyncWrite + AsyncRead + Unpin,
    InputFrame: DeserializeOwned,
    OutputFrame: Serialize,
{
    pub fn new(socket: Socket, buffer_capacity: usize) -> Self {
        Self {
            stream: BufWriter::new(socket),
            buffer: Vec::with_capacity(buffer_capacity),

            _input_frame_type: std::marker::PhantomData,
            _output_frame_type: std::marker::PhantomData,
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<InputFrame>> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // On success, the number of bytes is returned. `0` indicates "end of stream".
            if 0 == self
                .stream
                .read_buf(&mut self.buffer)
                .await
                .map_err(Error::FailedToReadFromStream)?
            {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(Error::ConnectionReset);
                }
            }
        }
    }

    fn parse_frame(&mut self) -> Result<Option<InputFrame>> {
        let mut buf = Cursor::new(&self.buffer[..]);

        match rmp_serde::from_read::<_, InputFrame>(&mut buf) {
            Ok(frame) => {
                let nb_of_bytes_read = buf.position() as usize;
                self.buffer.drain(0..nb_of_bytes_read);

                Ok(Some(frame))
            }

            // The buffer does not contain a complete frame. This is not an error we just need to
            // keep reading
            Err(FrameDecodeError::InvalidDataRead(_)) => Ok(None),
            Err(FrameDecodeError::InvalidMarkerRead(_)) => Ok(None),

            Err(err) => Err(Error::FailedToDecodeFrame(err)),
        }
    }

    pub async fn write_frame(&mut self, frame: &OutputFrame) -> Result<()> {
        let encoded_frame = rmp_serde::to_vec(frame).map_err(Error::FailedToEncodeFrame)?;

        self.stream
            .write_all(&encoded_frame)
            .await
            .map_err(Error::FailedToWriteToStream)?;

        self.stream
            .flush()
            .await
            .map_err(Error::FailedToWriteToStream)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use serde::Deserialize;

    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    enum Status {
        None,
        Str(String),
        Nb(u32),
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Frame1 {
        id: u32,
        name: String,
        status: Status,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
    struct Frame2 {
        name: String,
        status: Status,
    }

    #[tokio::test]
    async fn test_connection() {
        let (client, server) = tokio::io::duplex(1024);
        let mut client = Connection::<_, Frame1, Frame1>::new(client, 1024);
        let mut server = Connection::<_, Frame1, Frame1>::new(server, 1024);

        let frame_1 = Frame1 {
            id: 42,
            name: "test".to_string(),
            status: Status::Nb(10),
        };
        server.write_frame(&frame_1).await.unwrap();

        let frame_2 = Frame1 {
            id: 48,
            name: "".to_string(),
            status: Status::None,
        };
        server.write_frame(&frame_2).await.unwrap();

        assert_eq!(frame_1, client.read_frame().await.unwrap().unwrap());
        assert_eq!(frame_2, client.read_frame().await.unwrap().unwrap());

        drop(server);
        assert_eq!(None, client.read_frame().await.unwrap());
    }

    #[tokio::test]
    async fn test_connection_different_input_and_output_frames() {
        let (client, server) = tokio::io::duplex(1024);
        let mut client = Connection::<_, Frame1, Frame2>::new(client, 1024);
        let mut server = Connection::<_, Frame2, Frame1>::new(server, 1024);

        let frame_1 = Frame1 {
            id: 42,
            name: "test".to_string(),
            status: Status::None,
        };
        server.write_frame(&frame_1).await.unwrap();

        let frame_2 = Frame2 {
            name: "".to_string(),
            status: Status::Nb(10),
        };
        client.write_frame(&frame_2).await.unwrap();

        assert_eq!(frame_1, client.read_frame().await.unwrap().unwrap());
        assert_eq!(frame_2, server.read_frame().await.unwrap().unwrap());

        drop(client);
        assert_eq!(None, server.read_frame().await.unwrap());
    }

    //#[tokio::test]
    //async fn test_bad_data() {
    //    let (client, server) = tokio::io::duplex(1024);
    //    let mut client = Connection::<_, Frame1, Frame1>::new(client, 1024);
    //    let mut server = Connection::<_, Frame1, Frame1>::new(server, 1024);
    //}
    //
}
