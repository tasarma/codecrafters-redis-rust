use crate::resp::types::{RESPError, RESPValueRef};
use bytes::Bytes;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

type EXPIRE = u128;

#[derive(Debug, Clone)]
pub enum RedisCommand {
    Ping,
    Echo(Bytes),
    Set(Bytes, Bytes, Option<EXPIRE>),
    Get(Bytes),
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct StoredValue {
    pub value: Bytes,
    pub expiry: Option<EXPIRE>,
}

#[derive(Debug, Clone, Copy)]
enum ExpiryOption {
    Px(EXPIRE), // Relative expiry in milliseconds
}

pub type Store = Arc<Mutex<HashMap<Bytes, StoredValue>>>;

impl RedisCommand {
    pub fn resp_to_command(resp_data: &RESPValueRef, _store: Store) -> Result<Self, String> {
        let elements = match resp_data {
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

    pub fn execute(&self, store: &Store) -> Result<RESPValueRef, String> {
        match self {
            RedisCommand::Ping => Ok(RESPValueRef::SimpleString(Bytes::from_static(b"PONG"))),
            RedisCommand::Echo(data) => Ok(RESPValueRef::BulkString(data.clone())),
            RedisCommand::Set(key, value, expiry) => {
                store.lock().unwrap().insert(
                    key.clone(),
                    StoredValue {
                        value: value.clone(),
                        expiry: *expiry,
                    },
                );
                Ok(RESPValueRef::SimpleString(Bytes::from_static(b"OK")))
            }
            RedisCommand::Get(key) => {
                let current_time_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| "System time error".to_string())?
                    .as_millis();

                let mut store = store.lock().unwrap();
                match store.get(key) {
                    Some(stored) => {
                        //Ok(RESPValueRef::BulkString(value.clone())),
                        if let Some(expiry_time) = stored.expiry {
                            if current_time_ms >= expiry_time {
                                store.remove(key);
                                Ok(RESPValueRef::NullBulkString)
                            } else {
                                Ok(RESPValueRef::BulkString(stored.value.clone()))
                            }
                        } else {
                            Ok(RESPValueRef::BulkString(stored.value.clone()))
                        }
                    }
                    None => Ok(RESPValueRef::NullBulkString),
                }
            }
            RedisCommand::Unknown(cmd) => {
                let msg = format!("ERR unknown command '{}'", cmd);
                Ok(RESPValueRef::Error(Bytes::from(msg.into_bytes())))
            }
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

        let (key, value) = match (&elements[1], &elements[2]) {
            (RESPValueRef::BulkString(k), RESPValueRef::BulkString(v)) => (k.clone(), v.clone()),
            _ => return Err("SET key and value must be bulk strings".to_string()),
        };

        let expiry = if elements.len() == 5 {
            Some(Self::parse_expirty_option(&elements[3..])?)
        } else {
            None
        };

        Ok(RedisCommand::Set(key, value, expiry))
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

    fn parse_expirty_option(elements: &[RESPValueRef]) -> Result<EXPIRE, String> {
        if elements.len() < 2 {
            return Err("Expiry option requires a value".to_string());
        }

        let option = match &elements[0] {
            RESPValueRef::BulkString(bytes) => std::str::from_utf8(bytes)
                .map(|s| s.to_ascii_lowercase())
                .map_err(|_| "Invalid UTF-8 in expiry option".to_string())?,
            _ => return Err("Expiry option must be a bulk string".to_string()),
        };

        let expiry_value = match &elements[1] {
            RESPValueRef::BulkString(bytes) => std::str::from_utf8(bytes)
                .map_err(|_| "Invalid UTF-8 in expiry value".to_string())?
                .parse::<EXPIRE>()
                .map_err(|_| format!("Invalid {} value", option))?,
            _ => return Err("Expiry value must be a bulk string".to_string()),
        };

        let current_time_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "System time error".to_string())?
            .as_millis();

        match option.as_str() {
            "px" => Ok(current_time_ms + expiry_value),
            _ => Err(format!("Unknown SET option '{}'", option)),
        }
    }
}
