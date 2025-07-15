#![allow(unused_imports)]
use crate::{
    commands::RedisCommand,
    resp::{RESPError, RESPValueRef, RespParser},
};

use bytes::Bytes;
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::{
    collections::HashMap,
    error::Error,
    str,
    sync::{Arc, Mutex},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_util::codec::{Decoder, Framed};

const BIND_ADDRESS: &str = "127.0.0.1:6379";

type Store = Arc<Mutex<HashMap<bytes::Bytes, bytes::Bytes>>>;

pub async fn start_server() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(BIND_ADDRESS).await?;
    let store: Store = Arc::new(Mutex::new(HashMap::new()));

    println!("Redis server listening on {}", BIND_ADDRESS);

    loop {
        let (socket, _) = listener.accept().await?;
        let store_clone = Arc::clone(&store);
        tokio::spawn(async move {
            handle_client(socket, store_clone).await;
        });
    }
}

async fn handle_client(socket: TcpStream, store: Store) {
    let framed = Framed::new(socket, RespParser);
    let (mut writer, mut reader) = framed.split();

    while let Some(Ok(value)) = reader.next().await {
        println!("Received: {:?}", value);

        match RedisCommand::from_resp_array(&value, store.clone()) {
            Ok(command) => {
                if let Ok(response) = command.execute(&store) {
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
