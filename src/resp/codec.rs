use bytes::{BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use super::parsers::parse;
use super::{RESPError, RESPValueRef};

#[derive(Default)]
pub struct RespParser;

impl Decoder for RespParser {
    type Item = RESPValueRef;
    type Error = RESPError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        match parse(src, 0)? {
            Some((pos, value)) => {
                // We parsed a value! Shave off the bytes so tokio can continue filling the buffer.
                let our_data: BytesMut = src.split_to(pos);
                // Convert BytesMut into Bytes
                let data: Bytes = our_data.freeze();
                // Use `into_redis_value` to get the correct type
                let value = value.into_redis_value(&data);
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
}

impl Encoder<RESPValueRef> for RespParser {
    type Error = RESPError;

    fn encode(&mut self, item: RESPValueRef, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            // +OK\r\n
            RESPValueRef::SimpleString(bytes) => {
                dst.reserve(1 + bytes.len() + 2);
                dst.put_u8(b'+');
                dst.extend_from_slice(&bytes);
                dst.extend_from_slice(b"\r\n");
            }

            // -ERROR\r\n
            RESPValueRef::Error(bytes) => {
                dst.reserve(1 + bytes.len() + 2);
                dst.put_u8(b'-');
                dst.extend_from_slice(&bytes);
                dst.extend_from_slice(b"\r\n");
            }

            // :1234\r\n
            RESPValueRef::Int(i) => {
                let s = i.to_string();
                dst.reserve(1 + s.len() + 2);
                dst.put_u8(b':');
                dst.extend_from_slice(s.as_bytes());
                dst.extend_from_slice(b"\r\n");
            }

            // $3\r\nfoo\r\n
            RESPValueRef::BulkString(bytes) => {
                let len = bytes.len();
                let len_str = len.to_string();
                dst.reserve(1 + len_str.len() + 2 + len + 2); // $ + len + \r\n + data + \r\n
                dst.put_u8(b'$');
                dst.extend_from_slice(len_str.as_bytes());
                dst.extend_from_slice(b"\r\n");
                dst.extend_from_slice(&bytes);
                dst.extend_from_slice(b"\r\n");
            }

            // $-1\r\n
            RESPValueRef::NullBulkString => {
                dst.reserve(5);
                dst.extend_from_slice(b"$-1\r\n");
            }

            // *2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n
            RESPValueRef::Array(items) => {
                let len = items.len();
                let len_str = len.to_string();
                dst.reserve(1 + len_str.len() + 2); // * + len + \r\n
                dst.put_u8(b'*');
                dst.extend_from_slice(len_str.as_bytes());
                dst.extend_from_slice(b"\r\n");

                for item in items {
                    self.encode(item, dst)?;
                }
            }

            // *-1\r\n
            RESPValueRef::NullArray => {
                dst.reserve(5);
                dst.extend_from_slice(b"*-1\r\n");
            }
        }

        Ok(())
    }
}
