use crate::resp::types::{RESPError, RESPValueRef};
use bytes::Bytes;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub enum RedisCommand {
    Ping,
    Echo(Bytes),
    Set(Bytes, Bytes),
    Get(Bytes),
    Unknown(String),
}

// Shared state for key-value storage
pub type Store = Arc<Mutex<HashMap<Bytes, Bytes>>>;

impl RedisCommand {
    pub fn from_resp_array(array: &RESPValueRef, _store: Store) -> Result<Self, String> {
        let elements = match array {
            RESPValueRef::Array(ref arr) if !arr.is_empty() => arr,
            _ => return Err("Expected an array".to_string()),
        };

        let cmd = match &elements[0] {
            RESPValueRef::BulkString(bytes) => match std::str::from_utf8(bytes) {
                Ok(s) => s.to_ascii_lowercase(),
                Err(_) => return Err("Invalid UTF-8 in command name".to_string()),
            },
            _ => return Err("First element must be a bulk string".to_string()),
        };

        match cmd.as_str() {
            "ping" => Ok(RedisCommand::Ping),
            "echo" => {
                if elements.len() < 2 {
                    return Err("Missing argument for ECHO".to_string());
                }

                match &elements[1] {
                    RESPValueRef::BulkString(bytes) => Ok(RedisCommand::Echo(bytes.clone())),
                    _ => Err("ECHO argument must be a bulk string".to_string()),
                }
            }
            "set" => {
                if elements.len() < 3 {
                    return Err("Missing argument for SET".to_string());
                }

                if let (RESPValueRef::BulkString(key), RESPValueRef::BulkString(value)) =
                    (&elements[1], &elements[2])
                {
                    Ok(RedisCommand::Set(key.clone(), value.clone()))
                } else {
                    Err("SET key and value must be bulk strings".to_string())
                }
            }
            "get" => {
                if elements.len() < 2 {
                    return Err("Missing argument for GET".to_string());
                }

                if let RESPValueRef::BulkString(key) = &elements[1] {
                    Ok(RedisCommand::Get(key.clone()))
                } else {
                    Err("GET key must be bulk strings".to_string())
                }
            }
            unknown => Ok(RedisCommand::Unknown(unknown.to_string())),
        }
    }

    pub fn execute(&self, store: &Store) -> Result<RESPValueRef, String> {
        match self {
            RedisCommand::Ping => Ok(RESPValueRef::SimpleString(Bytes::from_static(b"PONG"))),
            RedisCommand::Echo(data) => Ok(RESPValueRef::BulkString(data.clone())),
            RedisCommand::Set(key, value) => {
                store.lock().unwrap().insert(key.clone(), value.clone());
                Ok(RESPValueRef::SimpleString(Bytes::from_static(b"OK")))
            }
            RedisCommand::Get(key) => {
                let store = store.lock().unwrap();
                match store.get(key) {
                    Some(value) => Ok(RESPValueRef::BulkString(value.clone())),
                    None => Ok(RESPValueRef::NullBulkString),
                }
            }
            RedisCommand::Unknown(cmd) => {
                let msg = format!("ERR unknown command '{}'", cmd);
                Ok(RESPValueRef::Error(Bytes::from(msg.into_bytes())))
            }
        }
    }
}
