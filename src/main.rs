#![allow(unused_imports)]
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::str;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_util::codec::{Decoder, Framed};

mod resp;
use resp::{RESPError, RESPValueRef, RespParser};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    println!("Listening on 127.0.0.1:6379");

    while let Ok((socket, _)) = listener.accept().await {
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
            Ok(bytes_read) => {
                let data = &buffer[..bytes_read];
                println!("data {:?}", str::from_utf8(data));

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
