
use bytes::{Buf, Bytes, BytesMut};
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};

use crate::{HttpHeader, map_str};

#[derive(Debug)]
pub struct HttpRequest {
    pub req_line: HttpReqLine,
    pub headers: HttpHeader,
    pub body: Option<Bytes>,
}
impl HttpRequest {
    pub fn new(req_line: HttpReqLine, headers: HttpHeader, body: Option<Bytes>) -> Self {
        Self {
            req_line,
            headers,
            body,
        }
    }
}

#[derive(Debug)]
pub struct HttpReqLine {
    pub method: String,
    pub url: String,
    pub version: String,
}
impl HttpReqLine {
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut head_line = s.split_whitespace();
        let method = head_line
            .next()
            .ok_or("method parsing error".to_string())?
            .to_string();
        let url = head_line
            .next()
            .ok_or("url parsing error".to_string())?
            .to_string();
        let version = head_line
            .next()
            .ok_or("version parsing error".to_string())?
            .to_string();
        Ok(Self {
            method,
            url,
            version,
        })
    }
}

pub async fn parse_http_frame(
    r: &mut OwnedReadHalf,
    buf: &mut BytesMut,
) -> Result<HttpRequest, String> {
    let (l, h) = parse_line_header_frame(r, buf).await?;
    let mut req = HttpRequest::new(l, h, None);
    if let Some(len) = req.headers.get("content-length") {
        let len = len.parse::<usize>().map_err(map_str!())?;
        parse_body_frame(len, r, buf).await?;
        let body = buf.split_to(len).freeze();
        req.body = Some(body);
    }
    Ok(req)
}

async fn parse_body_frame(
    len: usize,
    r: &mut OwnedReadHalf,
    buf: &mut BytesMut,
) -> Result<(), String> {
    loop {
        if buf.len() >= len {
            break;
        }
        if let Ok(len) = r.read_buf(buf).await {
            if len == 0 {
                return Err("other side closed".to_string());
            }
        } else {
            return Err("error!".to_string());
        }
    }
    Ok(())
}
async fn parse_line_header_frame(
    r: &mut OwnedReadHalf,
    buf: &mut BytesMut,
) -> Result<(HttpReqLine, HttpHeader), String> {
    loop {
        if let Ok(read_len) = r.read_buf(buf).await {
            if read_len == 0 {
                return Err("other side closed".to_string());
            }
            let (check_header_is_ok, position) = check_header(buf.chunk());
            if check_header_is_ok {
                let (l, h) = parse_line_header(&buf[..position])?;

                buf.advance(position);
                return Ok((l, h));
            }
        } else {
            return Err("error!".to_string());
        }
    }
}

fn parse_line_header(raw_header: &[u8]) -> Result<(HttpReqLine, HttpHeader), String> {
    let raw_header = str::from_utf8(raw_header).map_err(map_str!())?;
    let mut raw_header = raw_header.split("\r\n");
    let req_line = HttpReqLine::parse(
        raw_header
            .next()
            .ok_or("parse req line error".to_string())?,
    )?;
    let mut http_header = HttpHeader::new();
    for h in raw_header {
        if !h.is_empty() {
            http_header.parse_new_header(h)?;
        }
    }
    Ok((req_line, http_header))
}

fn check_header(c: &[u8]) -> (bool, usize) {
    for i in 0..=c.len() - 4 {
        if &c[i..i + 4] == b"\r\n\r\n" {
            return (true, i + 4);
        }
    }
    (false, 0)
}
