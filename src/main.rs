#![allow(unused_imports)]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            handle_connection(socket).await;
        });
    }
}

async fn handle_connection(mut socket: TcpStream) {
    let mut buffer = [0u8; 1024];

    loop {
        match socket.read(&mut buffer).await {
            Ok(0) => break,
            Ok(_) => {
                if let Err(e) = socket.write_all(b"+PONG\r\n").await {
                    eprintln!("Failed to write to socket: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("Failed to read from socket: {}", e);
                break;
            }
        }
    }
}
