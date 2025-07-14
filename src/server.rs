#![allow(unused_imports)]
use crate::{
    commands::RedisCommand,
    resp::{RESPError, RESPValueRef, RespParser},
};

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

    while let Some(Ok(value)) = reader.next().await {
        println!("Received: {:?}", value);

        match RedisCommand::from_resp_array(&value) {
            Ok(command) => {
                if let Ok(response) = command.execute() {
                    let _ = writer.send(response).await;
                }
            }
            Err(e) => {
                let _ = writer
                    .send(RESPValueRef::Error(Bytes::from(format!("ERR {}", e))))
                    .await;
            }
        }
    }
}
