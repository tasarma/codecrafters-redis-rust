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

#[derive(Debug, Clone)]
pub struct StoredValue {
    pub value: Bytes,
    pub expiry: Option<u64>,
}

pub type Store = Arc<Mutex<HashMap<Bytes, Bytes>>>;

impl RedisCommand {
    pub fn resp_to_command(array: &RESPValueRef, _store: Store) -> Result<Self, String> {
        let elements = match array {
            RESPValueRef::Array(ref arr) if !arr.is_empty() => arr,
            _ => return Err("Expected an array".to_string()),
        };

        let cmd = match elements.first() {
            Some(RESPValueRef::BulkString(bytes)) => std::str::from_utf8(bytes)
                .map(|s| s.to_ascii_lowercase())
                .map_err(|_| "Invalid UTF-8 in command name".to_string())?,
            _ => return Err("First element must be a bulk string".to_string()),
        };

        match cmd.as_str() {
            "ping" => Ok(RedisCommand::Ping),
            "echo" => Self::parse_echo(elements),
            "set" => Self::parse_set(elements),
            "get" => Self::parse_get(elements),
            unknown => Ok(RedisCommand::Unknown(unknown.to_string())),
        }
    }

    fn parse_echo(elements: &[RESPValueRef]) -> Result<Self, String> {
        if elements.len() < 2 {
            return Err("Missing argument for ECHO".to_string());
        }

        match &elements[1] {
            RESPValueRef::BulkString(bytes) => Ok(RedisCommand::Echo(bytes.clone())),
            _ => Err("ECHO argument must be a bulk string".to_string()),
        }
    }

    fn parse_set(elements: &[RESPValueRef]) -> Result<Self, String> {
        if elements.len() < 3 {
            return Err("Missing argument for SET".to_string());
        }

        if let (RESPValueRef::BulkString(key), RESPValueRef::BulkString(value)) =
            (&elements[1], &elements[2])
        {
            Ok(RedisCommand::Set(key.clone(), value.clone()))
        } else {
            Err("SET values must be bulk strings".to_string())
        }
    }
    fn parse_get(elements: &[RESPValueRef]) -> Result<Self, String> {
        if elements.len() < 2 {
            return Err("Missing argument for GET".to_string());
        }

        if let RESPValueRef::BulkString(key) = &elements[1] {
            Ok(RedisCommand::Get(key.clone()))
        } else {
            Err("GET key must be bulk strings".to_string())
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
