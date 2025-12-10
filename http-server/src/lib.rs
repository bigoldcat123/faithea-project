use std::collections::{HashMap};

use bytes::{BufMut, Bytes, BytesMut};

pub mod request;
pub mod response;
pub mod server;
pub mod handler;

#[macro_export]
macro_rules! map_str {
    () => {
        |x| format!("{}", x)
    };
}

#[derive(Debug, Default)]
pub struct HttpHeader {
    headers: HashMap<String, String>,
}
impl HttpHeader {
    pub fn new() -> Self {
        Self {
            headers: HashMap::new(),
        }
    }
    pub fn parse_new_header(&mut self, s: &str) -> Result<(), String> {
        let mut k_v = s.split(":");
        let k = k_v
            .next()
            .ok_or("no key".to_string())?
            .trim()
            .to_lowercase();
        let v = k_v
            .next()
            .ok_or("no value".to_string())?
            .trim()
            .to_lowercase();
        self.headers.insert(k, v);
        Ok(())
    }
    pub fn get(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    fn add(&mut self, key: &str, value: &str) {
        self.headers.insert(key.into(), value.into());
    }
}

impl From<&HttpHeader> for Bytes {
    fn from(value: &HttpHeader) -> Self {
        let mut b = BytesMut::with_capacity(256);
        for (k, v) in value.headers.iter() {
            b.put(format!("{k}:{v}\r\n").as_bytes());
        }
        b.put("\r\n".as_bytes());

        b.freeze()
    }
}




#[cfg(test)]
mod test {

    use bytes::{Buf, Bytes};

    use crate::HttpHeader;

    #[test]
    fn into_bytes_test() {
        let mut header = HttpHeader::new();
        // some real HTTP headers
        header.add("Host", "example.com");
        header.add("User-Agent", "rust-test/0.1");
        header.add("Accept", "*/*");
        header.add("Connection", "close");

        let bytes: Bytes = (&header).into();
        let s = std::str::from_utf8(bytes.chunk()).unwrap();
        // Split into lines (ignore the final empty line caused by the trailing \r\n\r\n),
        // sort to avoid depending on HashMap iteration order, and compare.
        let mut got: Vec<&str> = s.split("\r\n").filter(|l| !l.is_empty()).collect();
        got.sort();
        let mut expected = vec![
            "Host:example.com",
            "User-Agent:rust-test/0.1",
            "Accept:*/*",
            "Connection:close",
        ];
        expected.sort();
        assert_eq!(got, expected);
        assert!(s.ends_with("\r\n\r\n"));
    }
}
