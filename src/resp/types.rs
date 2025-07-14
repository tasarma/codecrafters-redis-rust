use bytes::{Bytes, BytesMut};

pub struct BufSplit(pub usize, pub usize);

impl BufSplit {
    #[inline]
    pub fn as_slice<'a>(&self, buf: &'a BytesMut) -> &'a [u8] {
        &buf[self.0..self.1]
    }

    #[inline]
    fn as_bytes(&self, buf: &Bytes) -> Bytes {
        buf.slice(self.0..self.1)
    }
}

pub enum RESPBufSplit {
    String(BufSplit),
    Error(BufSplit),
    Int(i64),
    Array(Vec<RESPBufSplit>),
    BulkString(BufSplit),
    NullArray,
    NullBulkString,
}

impl RESPBufSplit {
    pub fn into_redis_value(self, buf: &Bytes) -> RESPValueRef {
        match self {
            RESPBufSplit::String(bfs) => RESPValueRef::SimpleString(bfs.as_bytes(buf)),
            RESPBufSplit::Error(bfs) => RESPValueRef::Error(bfs.as_bytes(buf)),
            RESPBufSplit::Int(i) => RESPValueRef::Int(i),
            RESPBufSplit::BulkString(bfs) => RESPValueRef::BulkString(bfs.as_bytes(buf)),
            RESPBufSplit::NullBulkString => RESPValueRef::NullBulkString,
            RESPBufSplit::Array(arr) => RESPValueRef::Array(
                arr.into_iter()
                    .map(|bfs| bfs.into_redis_value(buf))
                    .collect(),
            ),
            RESPBufSplit::NullArray => RESPValueRef::NullArray,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum RESPValueRef {
    SimpleString(Bytes),
    Error(Bytes),
    Int(i64),
    BulkString(Bytes),
    Array(Vec<RESPValueRef>),
    NullArray,
    NullBulkString,
}

#[derive(thiserror::Error, Debug)]
pub enum RESPError {
    #[error("Unexpected end of input")]
    UnexpectedEnd,

    #[error("Unknown starting byte")]
    UnknownStartingByte,

    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Failed to parse integer")]
    IntParseFailure,

    #[error("Invalid bulk string size: {0}")]
    BadBulkStringSize(i64),

    #[error("Invalid array size: {0}")]
    BadArraySize(i64),
}
