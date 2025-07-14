use crate::resp::types::{RESPError, RESPValueRef};
use bytes::Bytes;

#[derive(Debug, Clone)]
pub enum RedisCommand {
    Ping,
    Echo(Bytes),
    Unknown(String),
}

impl RedisCommand {
    pub fn from_resp_array(array: &RESPValueRef) -> Result<Self, String> {
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
                    RESPValueRef::BulkString(data) => Ok(RedisCommand::Echo(data.clone())),
                    _ => Err("ECHO argument must be a bulk string".to_string()),
                }
            }
            unknown => Ok(RedisCommand::Unknown(unknown.to_string())),
        }
    }

    pub fn execute(&self) -> Result<RESPValueRef, String> {
        match self {
            RedisCommand::Ping => Ok(RESPValueRef::SimpleString(Bytes::from_static(b"PONG"))),
            RedisCommand::Echo(data) => Ok(RESPValueRef::BulkString(data.clone())),
            RedisCommand::Unknown(cmd) => {
                let msg = format!("ERR unknown command '{}'", cmd);
                Ok(RESPValueRef::Error(Bytes::from(msg.into_bytes())))
            }
        }
    }
}
