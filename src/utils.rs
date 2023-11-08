use crate::message::Message;
use std::io;
use tokio::net::UdpSocket; // We use the socket type from tokio, not std's.

pub async fn send_msg(sock: &UdpSocket, msg: Message) -> io::Result<()> {
    // Serialize the message to a JSON string.
    let json_text: String = serde_json::to_string(&msg).unwrap();

    // We will create a payload in this format. It starts with a
    // 4-byte integer, which is the message length of the following
    // JSON text.
    //
    // offset | 0..4        | 4...      |
    // fields | length: u32 | JSON text |

    // Re-interpret the JSON string as bytes.
    let json_bytes: &[u8] = json_text.as_bytes();

    // Get the payload length and create the 4-byte header.
    let len: usize = json_bytes.len();
    let len: u32 = len as u32;
    let len_bytes: [u8; 4] = len.to_le_bytes();

    // Send the 4-byte length.
    send_exact(sock, &len_bytes).await?;

    // Send the JSON bytes.
    send_exact(sock, json_bytes).await?;

    Ok(())
}

/// Sends whole buffer to the socket.
async fn send_exact(sock: &UdpSocket, buf: &[u8]) -> io::Result<()> {
    // `rest` points to the remaining sub-slice that is not sent yet.
    let mut rest = buf;

    // Loops until the rest unsent bytes become empty.
    while !rest.is_empty() {
        // Send the remaining bytes and returns the actual number of
        // sent bytes.
        let count = sock.send(rest).await?;

        // It's a special case when the socket is closed. Here returns
        // an error because the current function expects that the
        // whole buffer should be sent.
        if count == 0 {
            let err = io::Error::new(io::ErrorKind::ConnectionAborted, "The socket is closed");
            return Err(err);
        }

        // Forward the `rest` by `count` bytes.
        rest = &rest[count..];
    }

    Ok(())
}

/// The async function tries to read a message from the UDP socket.
///
/// If one message is successfully received and decoded, it returns
/// Some(msg). If the socket is closed, it returns None.
pub async fn recv_msg(sock: &UdpSocket) -> io::Result<Message> {
    // Here it reads 4 bytes from the socket to learn the length of
    // the following JSON bytes.

    let mut len_bytes = [0u8; 4]; // Creates a 4-byte buffer.
    recv_exact(sock, &mut len_bytes).await?; // Fill the bytes in the buffer from the socket.

    // Convert the bytes to an integer.
    let len: u32 = u32::from_le_bytes(len_bytes);
    let len = len as usize;

    // Next, read the following JSON bytes.

    // Creates a buffer to store JSON bytes. Here we use a Vec instead
    // of an array because the size is determined in runtime.
    let mut json_bytes = vec![0u8; len];
    recv_exact(sock, &mut json_bytes).await?;

    // Convert the JSON bytes to a JSON string.
    let json_text = String::from_utf8(json_bytes).unwrap();

    // Decode the JSON string into a message.
    let msg: Message = serde_json::from_str(&json_text).unwrap();

    Ok(msg)
}

/// It reads bytes from the socket until the whole buffer is full.
async fn recv_exact(sock: &UdpSocket, buf: &mut [u8]) -> io::Result<()> {
    // The `rest` is a sub-slice of `buf`, pointing to the tailing
    // bytes that are not filled in yet.
    let mut rest = buf;

    // Loop when the `rest` is not empty.
    while !rest.is_empty() {
        // Read bytes from the socket. It returns the number of
        // received bytes.
        let count = sock.recv(rest).await?;

        // It is a special case when the socket is closed.  It returns
        // a error because the function expects that the buffer must
        // be full-filled, but the socket closes early.
        if count == 0 {
            let err = io::Error::new(io::ErrorKind::UnexpectedEof, "The socket is closed");
            return Err(err);
        }

        // Forward the `rest` by `count` bytes. It's done by taking a
        // sub-slice of itself.
        rest = &mut rest[count..];
    }

    Ok(())
}
