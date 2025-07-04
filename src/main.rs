#![allow(unused_imports)]
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use tokio::stream;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handle_client(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buf = [0u8; 1024];

    loop {
        let bytes_read = stream.read(&mut buf).unwrap();

        if bytes_read == 0 {
            break;
        }

        stream.write_all(b"+PONG\r\n").unwrap();
    }
}
