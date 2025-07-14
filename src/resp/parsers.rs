use std::str::from_utf8;

use bytes::{Bytes, BytesMut};
use memchr::memchr;

use super::types::{BufSplit, RESPBufSplit, RESPError, RESPValueRef};

type RESPResult = Result<Option<(usize, RESPBufSplit)>, RESPError>;

// Tries to extract a word from the current position.
// A word ends at `\r\n` and excludes those delimiters.
#[inline]
pub fn word(buf: &BytesMut, cursor: usize) -> Option<(usize, BufSplit)> {
    // Ensure the start position is within the buffer bounds
    if buf.len() <= cursor {
        return None;
    }

    // Search for '\r' starting from the given position
    memchr(b'\r', &buf[cursor..]).and_then(|end| {
        if end + 1 < buf.len() && buf[cursor + end + 1] == b'\n' {
            // Return the position after '\r\n' and the word boundaries
            Some((cursor + end + 2, BufSplit(cursor, cursor + end)))
        } else {
            // Incomplete CLRF
            None
        }
    })
}

// Simple String (+...)
pub fn simple_string(buf: &BytesMut, cursor: usize) -> RESPResult {
    Ok(word(buf, cursor).map(|(next_cursor, split)| (next_cursor, RESPBufSplit::String(split))))
}

// Error (-...)
pub fn error(buf: &BytesMut, cursor: usize) -> RESPResult {
    Ok(word(buf, cursor).map(|(next_cursor, split)| (next_cursor, RESPBufSplit::Error(split))))
}

// Int (:...)
fn int(buf: &BytesMut, cursor: usize) -> Result<Option<(usize, i64)>, RESPError> {
    match word(buf, cursor) {
        Some((next_cursor, split)) => {
            let word = from_utf8(split.as_slice(buf)).map_err(|_| RESPError::IntParseFailure)?;
            let num = word
                .parse::<i64>()
                .map_err(|_| RESPError::IntParseFailure)?;
            Ok(Some((next_cursor, num)))
        }
        None => Ok(None),
    }
}

pub fn resp_int(buf: &BytesMut, cursor: usize) -> RESPResult {
    Ok(int(buf, cursor)?.map(|(next_cursor, num)| (next_cursor, RESPBufSplit::Int(num))))
}

// Bulkstring($...)
pub fn bulk_string(buf: &BytesMut, cursor: usize) -> RESPResult {
    match int(buf, cursor)? {
        // redis defines a NullBulkString type, with length of -1.
        Some((next_cursor, -1)) => Ok(Some((next_cursor, RESPBufSplit::NullBulkString))),
        Some((next_cursor, size)) if size >= 0 => {
            let total_size = next_cursor + size as usize;
            // The client hasn't sent us enough bytes
            if buf.len() < total_size + 2 {
                Ok(None)
            } else {
                // total_size + 2 == ...bulkstring\r\n<HERE> -- after CLRF
                Ok(Some((
                    total_size + 2,
                    RESPBufSplit::BulkString(BufSplit(next_cursor, total_size)),
                )))
            }
        }
        // We recieved a garbage size (size < -1), so error out
        Some((_, bad_size)) => Err(RESPError::BadBulkStringSize(bad_size)),
        None => Ok(None),
    }
}

pub fn parse(buf: &BytesMut, cursor: usize) -> RESPResult {
    if buf.is_empty() || cursor >= buf.len() {
        return Ok(None);
    }

    match buf[cursor] {
        b'+' => simple_string(buf, cursor + 1),
        b'-' => error(buf, cursor + 1),
        b':' => resp_int(buf, cursor + 1),
        b'$' => bulk_string(buf, cursor + 1),
        b'*' => array(buf, cursor + 1),
        _ => Err(RESPError::UnknownStartingByte),
    }
}

pub fn array(buf: &BytesMut, cursor: usize) -> RESPResult {
    match int(buf, cursor)? {
        None => Ok(None),
        // redis defines a NullArray type, with length of -1.
        Some((next_cursor, -1)) => Ok(Some((next_cursor, RESPBufSplit::NullArray))),
        Some((next_cursor, num_elements)) if num_elements >= 0 => {
            let mut values = Vec::with_capacity(num_elements as usize);
            let mut currsor_pos = next_cursor;

            for _ in 0..num_elements {
                match parse(buf, currsor_pos)? {
                    Some((new_cursor, value)) => {
                        currsor_pos = new_cursor;
                        values.push(value);
                    }
                    None => return Ok(None),
                }
            }
            Ok(Some((currsor_pos, RESPBufSplit::Array(values))))
        }
        Some((_cursor, bad_num_elements)) => Err(RESPError::BadArraySize(bad_num_elements)),
    }
}
