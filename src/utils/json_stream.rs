use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;

/// Read a JSON-serialized value from a TcpStream
pub fn read_json<T>(stream: &mut TcpStream, buffer_size: usize) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let mut buffer = vec![0u8; buffer_size];
    let size = stream.read(&mut buffer)?;
    let json_str = String::from_utf8_lossy(&buffer[..size]);
    serde_json::from_str(&json_str).with_context(|| format!("Failed to parse JSON: {}", json_str))
}

/// Write a JSON-serialized value to a TcpStream
pub fn write_json<T>(stream: &mut TcpStream, value: &T) -> Result<()>
where
    T: Serialize,
{
    let json_str = serde_json::to_string(value).context("Failed to serialize value to JSON")?;
    stream.write_all(json_str.as_bytes())?;
    stream.flush()?;
    Ok(())
}

/// Send a JSON request and read a JSON response over a TcpStream
pub fn send_json_request<Req, Resp>(
    stream: &mut TcpStream,
    request: &Req,
    buffer_size: usize,
) -> Result<Resp>
where
    Req: Serialize,
    Resp: for<'de> Deserialize<'de>,
{
    write_json(stream, request)?;
    read_json(stream, buffer_size)
}
