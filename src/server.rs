#![allow(unused_imports)]
use crate::resp::{RESPError, RESPValueRef, RespParser};
use bytes::Bytes;
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::error::Error;
use std::str;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_util::codec::{Decoder, Framed};

const BIND_ADDRESS: &str = "127.0.0.1:6379";

pub async fn start_server() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(BIND_ADDRESS).await?;
    println!("Redis server listening on {}", BIND_ADDRESS);

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            handle_client(socket).await;
        });
    }
}

async fn handle_client(mut socket: TcpStream) {
    let framed = Framed::new(socket, RespParser::default());
    let (mut writer, mut reader) = framed.split();

    while let Some(result) = reader.next().await {
        match result {
            Ok(value) => {
                println!("Received: {:?}", value);

                // Echo back a "+OK\r\n" simple string response
                let response = RESPValueRef::SimpleString(Bytes::from_static(b"OK"));

                if let Err(e) = writer.send(response).await {
                    eprintln!("Failed to send response: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("Parse error: {}", e);
                break;
            }
        }
    }
}
