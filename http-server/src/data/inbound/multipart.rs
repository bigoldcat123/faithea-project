use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use bytes::{Buf, Bytes, BytesMut};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::OwnedReadHalf,
};

use crate::{
    TryConvertFrom, map_str,
    request::{HttpRequest, RequestBody},
};

pub type MultipartDataMap = HashMap<String, Vec<Part>>;
/// macro generate!
pub trait TryFromMultipartDataMap: Sized {
    fn try_from_multipart_data_map(data: &mut MultipartDataMap) -> Result<Self, String>;
}

#[derive(Debug)]
pub enum Part {
    Lit(String),
    File(MultiPartFile),
}
macro_rules! impl_try_from_part_for_parse_from_str {
    ($($t:ty),*) => {
        $(
            impl TryFrom<Part> for $t {
                type Error = String;
                fn try_from(value: Part) -> Result<Self, Self::Error> {
                    if let Part::Lit(l) = value {
                        Ok(l.parse::<Self>().map_err(map_str!())?)
                    }else {
                        Err(format!("{} not compatiable to transform part to MultiPartFile",stringify!($t)))
                    }
                }
            }
        )*
    };
}

impl_try_from_part_for_parse_from_str!(
    i8, i16, i32, i64, i128, isize, usize, f32, f64, u8, u16, u32, u64, u128, bool, String
);

impl<T: TryFrom<Part, Error = String>> TryConvertFrom<Vec<Part>> for T {
    fn try_convert_from(mut value: Vec<Part>) -> Result<Self, String> {
        if let Some(value) = value.pop() {
            value.try_into()
        } else {
            Err("there is no data in multipart map".to_string())
        }
    }
}

impl<T: TryFrom<Part>> TryConvertFrom<Vec<Part>> for Vec<T> {
    fn try_convert_from(value: Vec<Part>) -> Result<Self, String> {
        Ok(value
            .into_iter()
            .filter_map(|x| T::try_from(x).ok())
            .collect())
    }
}

#[derive(Debug)]
pub struct MultiPartFile {
    pub file_name: Option<String>,
    pub temp_path: String,
    pub mime_type: Option<String>,
}

impl Drop for MultiPartFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.temp_path.as_str());
    }
}

impl TryFrom<Part> for MultiPartFile {
    type Error = String;
    fn try_from(value: Part) -> Result<Self, Self::Error> {
        if let Part::File(f) = value {
            Ok(f)
        } else {
            Err("not compatiable to transform part to MultiPartFile".to_string())
        }
    }
}

#[derive(Debug)]
pub struct Multipart<T: TryFromMultipartDataMap>(T);

impl<T: TryFromMultipartDataMap> Multipart<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: TryFromMultipartDataMap> Deref for Multipart<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: TryFromMultipartDataMap> DerefMut for Multipart<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: TryFromMultipartDataMap> TryFrom<&mut HttpRequest> for Multipart<T> {
    type Error = String;
    fn try_from(req: &mut HttpRequest) -> Result<Self, Self::Error> {
        match req.body.as_mut() {
            Some(RequestBody::MultiPart(body)) => {
                Ok(Multipart(T::try_from_multipart_data_map(body)?))
            }
            _ => Err("no boundary".into()),
        }
    }
}

enum MultiPartBodyParserState {
    Start,
    Header,
    Body,
    End,
}
#[derive(Default)]
struct HeaderInfo {
    name: Option<String>,
    mime_type: Option<String>,
    file_name: Option<String>,
}
pub struct MultiPartBodyParser<'a> {
    r: &'a mut OwnedReadHalf,
    buf: &'a mut BytesMut,
    len: usize,
    readed: usize,
    boundary: &'a [u8],
    boundary_with_prefix: Vec<u8>,
    boundary_with_prefix_next: Vec<usize>,
    map: Option<MultipartDataMap>,
    state: MultiPartBodyParserState,
    header_info: HeaderInfo,
}
impl<'a> MultiPartBodyParser<'a> {
    fn new_with_start_state(
        r: &'a mut OwnedReadHalf,
        buf: &'a mut BytesMut,
        len: usize,
        boundary: &'a [u8],
    ) -> Self {
        let map = Some(MultipartDataMap::new());
        let readed = 0;
        // let boundary_with_prefix = format!("\r\n--{boundary}");
        let mut boundary_with_prefix = BytesMut::new();
        boundary_with_prefix.extend_from_slice(b"\r\n--");
        boundary_with_prefix.extend_from_slice(boundary);
        let boundary_with_prefix = boundary_with_prefix.to_vec();
        let boundary_with_prefix_next = build_boundary_next_array(&boundary_with_prefix);
        let state = MultiPartBodyParserState::Start;
        let header_info = Default::default();
        Self {
            r,
            buf,
            len,
            readed,
            boundary,
            boundary_with_prefix,
            boundary_with_prefix_next,
            state,
            map,
            header_info,
        }
    }
    pub async fn parse(
        r: &'a mut OwnedReadHalf,
        buf: &'a mut BytesMut,
        len: usize,
        boundary: &'a str,
    ) -> Result<RequestBody, String> {
        let state_machine = Self::new_with_start_state(r, buf, len, boundary.as_bytes());
        state_machine.process().await
    }
    async fn process(mut self) -> Result<RequestBody, String> {
        use MultiPartBodyParserState::*;
        loop {
            match self.state {
                Start => {
                    self.remove_mutipart_body_prefix().await?;
                }
                Header => {
                    self.parse_header().await?;
                }
                Body => {
                    self.parse_body().await?;
                }
                End => return Ok(RequestBody::MultiPart(self.map.take().unwrap())),
            }
        }
    }

    fn is_file_body(&self) -> bool {
        self.header_info.file_name.is_some() || self.header_info.mime_type.is_some()
    }
    async fn parse_file_body(&mut self) -> Result<MultiPartFile, String> {
        let file_name = self.header_info.file_name.take();
        let mime_type = self.header_info.mime_type.take();
        let multipart_file = MultiPartFile {
            temp_path: format!(
                "/Users/dadigua/Desktop/graduation/temp{}",
                rand::random::<u64>()
            ),
            file_name,
            mime_type,
        };
        let mut f = tokio::fs::File::create(&multipart_file.temp_path)
            .await
            .map_err(|x| x.to_string())?;
        loop {
            while self.buf.len() < self.boundary.len() + 7 {
                let read_len = self.r.read_buf(self.buf).await.map_err(map_str!())?;
                if read_len == 0 {
                    return Err("Unexpected EOF".to_string());
                }
            }
            // let (is_ended, len) = check_body_end(self.buf,&self.boundary_with_prefix,&self.boundary_with_prefix_next);
            let (is_ended, len) = self.check_body_end();

            if is_ended {
                let mut b = self.buf.split_to(len);
                f.write_buf(&mut b).await.map_err(map_str!())?;
                let _ = self.buf.split_to(self.boundary.len() + 2 + 2 + 2);
                self.readed += self.boundary.len() + 2 + 2 + 2;
                self.readed += len;
                break;
            } else {
                let mut b = self.buf.split_to(self.buf.len() - self.boundary.len() - 6);
                self.readed += b.len();
                f.write_buf(&mut b).await.map_err(map_str!())?;
            }
        }
        //delete file
        Ok(multipart_file)
    }

    fn check_body_end(&self) -> (bool, usize) {
        let mut i = 0;
        let mut j = 0;
        while i < self.buf.len() {
            if self.boundary_with_prefix[j] == self.buf[i] {
                i += 1;
                j += 1;
            } else if j > 0 {
                j = self.boundary_with_prefix_next[j - 1];
            } else {
                i += 1;
            }
            if j == self.boundary_with_prefix.len()
                && i + 1 < self.buf.len()
                && ((self.buf[i] == b'\r' && self.buf[i + 1] == b'\n')
                    || (self.buf[i] == b'-' && self.buf[i + 1] == b'-'))
            {
                return (true, i - self.boundary_with_prefix.len());
            }
        }
        (false, 0)
    }

    async fn parse_lit_body(&mut self) -> Result<Bytes, String> {
        let mut simple_body = BytesMut::new();
        loop {
            while self.buf.len() <= self.boundary.len() + 6 {
                let read_len = self.r.read_buf(self.buf).await.map_err(map_str!())?;
                if read_len == 0 {
                    return Err("Unexpected EOF".to_string());
                }
            }
            // println!("-> {:?} {:?} {:?} {:?}", buf,name,file_name,mime_type);
            // let (is_ended, len) = check_body_end(self.buf,&self.boundary_with_prefix,&self.boundary_with_prefix_next);
            let (is_ended, len) = self.check_body_end();
            if is_ended {
                let b = self.buf.split_to(len);
                simple_body.extend_from_slice(&b[..]);
                // /r/n --boundary /r/n => 2 + 2 + 2 + len
                let _ = self.buf.split_to(self.boundary.len() + 2 + 2 + 2);
                self.readed += self.boundary.len() + 2 + 2 + 2;
                self.readed += len;
                break;
            } else {
                let b = self.buf.split_to(self.buf.len() - self.boundary.len() - 6);
                self.readed += self.buf.len() - self.boundary.len() - 6;
                simple_body.extend_from_slice(&b[..]);
            }
        }
        Ok(simple_body.freeze())
    }

    fn body_ends(&self) -> bool {
        &self.buf[..] == b"\r\n" && self.readed + 2 == self.len
    }
    async fn parse_body(&mut self) -> Result<(), String> {
        let key_name = self
            .header_info
            .name
            .take()
            .unwrap_or("default".to_string());
        if self.is_file_body() {
            let multipart_file = self.parse_file_body().await?;
            if let Some(map) = self.map.as_mut() {
                map.entry(key_name)
                    .or_default()
                    .push(Part::File(multipart_file));
            }
        } else {
            let a = self.parse_lit_body().await?;
            let data = String::from_utf8(a.to_vec()).map_err(map_str!())?;
            if let Some(map) = self.map.as_mut() {
                map.entry(key_name).or_default().push(Part::Lit(data));
            }
        }
        self.state = MultiPartBodyParserState::Header;
        if self.body_ends() {
            self.state = MultiPartBodyParserState::End;
            let _ = self.buf.split();
            self.readed += 2;
        }
        Ok(())
    }

    fn check_mutipart_header(&self) -> (bool, usize) {
        let next = [0, 0, 1, 0];
        let p = b"\r\n\r\n";
        let mut i = 0;
        let mut j = 0_usize;
        while i < self.buf.len() {
            if self.buf[i] == p[j] {
                i += 1;
                j += 1;
            } else if j > 0 {
                j = next[j - 1];
            } else {
                i += 1;
            }
            if j == 4 {
                return (true, i);
            }
        }
        (false, 0)
    }

    async fn parse_header(&mut self) -> Result<(), String> {
        let header_len;
        loop {
            let (inner_is_ok, inner_header_len) = self.check_mutipart_header();
            if inner_is_ok {
                header_len = inner_header_len;
                break;
            }
            let read_len = self.r.read_buf(self.buf).await.map_err(map_str!())?;
            if read_len == 0 {
                return Err("Unexpected EOF".to_string());
            }
        }
        self.readed += header_len;
        let b = self.buf.split_to(header_len);
        let b = str::from_utf8(&b).map_err(map_str!())?;
        for l in b.split("\r\n") {
            if !l.is_empty() {
                self.process_multipart_header(l);
            }
        }
        self.state = MultiPartBodyParserState::Body;
        Ok(())
    }
    fn process_multipart_header(&mut self, header_line: &str) {
        if let Some((k, v)) = header_line.split_once(":") {
            if k.eq_ignore_ascii_case("Content-Disposition") {
                for kv in v.split(";") {
                    if let Some((k, v)) = kv.split_once("=") {
                        if k.trim() == "name" {
                            self.header_info.name = Some(v[1..v.len() - 1].to_string());
                        }
                        if k.trim() == "filename" {
                            self.header_info.file_name = Some(v[1..v.len() - 1].to_string())
                        }
                    }
                }
            } else if k.eq_ignore_ascii_case("Content-Type") {
                self.header_info.mime_type = Some(v.trim().to_string())
            }
        }
    }
    async fn remove_mutipart_body_prefix(&mut self) -> Result<(), String> {
        // remove pre_fix
        while self.buf.len() < self.boundary.len() + 2 {
            let read_len = self.r.read_buf(self.buf).await.map_err(map_str!())?;
            if read_len == 0 {
                return Err("Unexpected EOF".to_string());
            }
        }
        if &self.buf[..2] != b"--"
            || &self.buf[2..2 + self.boundary.len()] != self.boundary
            || &self.buf[2 + self.boundary.len()..2 + self.boundary.len() + 2] != b"\r\n"
        {
            return Err("Invalid boundary".to_string());
        }
        self.buf.advance(2 + self.boundary.len() + 2);
        self.readed += 2 + self.boundary.len() + 2;
        self.state = MultiPartBodyParserState::Header;
        Ok(())
    }
}

fn build_boundary_next_array(boundary: &[u8]) -> Vec<usize> {
    let mut ans = vec![0; boundary.len()];
    let mut i = 1;
    let mut len = 0;
    while i < boundary.len() - 1 {
        if boundary[i] == boundary[len] {
            len += 1;
            ans[i] = len;
            i += 1;
        } else if len > 0 {
            len = ans[len - 1];
        } else {
            i += 1;
        }
    }
    ans
}

#[cfg(test)]
mod tests {
    // use bytes::BufMut;

    // use super::*;

    // fn check_mutipart_header2(buf: &[u8]) -> (bool, usize) {
    //     for i in 0..=buf.len() - 4 {
    //         if &buf[i..i + 4] == b"\r\n\r\n" {
    //             return (true, i + 4);
    //         }
    //     }
    //     (false, 0)
    // }
    // fn check_body_end2(buf: &mut BytesMut, boundary: &str) -> (bool, usize) {
    //     for i in 0..=buf.len() - boundary.len() - 2 - 2 - 2 {
    //         if &buf[i..i + 2] == b"\r\n"// 2
    //             && &buf[i + 2..i + 4] == b"--"// 4
    //             && &buf[i + 4..i + boundary.len() + 4] == boundary.as_bytes()// 4 +  len
    //             &&( &buf[i + boundary.len() + 4..i + boundary.len() + 6] == b"\r\n"|| &buf[i + boundary.len() + 4..i + boundary.len() + 6] == b"--")
    //         // 6 + len
    //         {
    //             return (true, i);
    //         }
    //     }
    //     (false, 0)
    // }
    // #[test]
    // fn check_mutipart_header_test2() {
    //     let a = "abcab";
    //     println!("{:?}", build_boundary_next_array(a));

    //     let boundary = "-------------asdjasujd";
    //     let boundary_with_prefix = format!("\r\n--{}", boundary);
    //     let mut buf = BytesMut::new();
    //     buf.put(&b"asjdilas\r\n---------------asdjasujd\r\nasda"[..]);
    //     let boundary_with_prefix_next = build_boundary_next_array(boundary_with_prefix.as_str());

    //     assert_eq!(
    //         check_body_end(
    //             &mut buf,
    //             boundary_with_prefix.as_str(),
    //             &boundary_with_prefix_next
    //         ),
    //         check_body_end2(&mut buf, boundary)
    //     );
    //     let boundary = "-------------asdjasujd";
    //     let boundary_with_prefix = format!("\r\n--{}", boundary);
    //     let mut buf = BytesMut::new();
    //     buf.put(&b"asjdilas\r\n---------------asdjasujd--asda"[..]);
    //     let boundary_with_prefix_next = build_boundary_next_array(boundary_with_prefix.as_str());

    //     assert_eq!(
    //         check_body_end(
    //             &mut buf,
    //             boundary_with_prefix.as_str(),
    //             &boundary_with_prefix_next
    //         ),
    //         check_body_end2(&mut buf, boundary)
    //     );
    // }

    // #[test]
    // fn check_mutipart_header_test() {
    //     let b = "jasldasdasdjasdhasdasdaskuashdj\r\nasd\r\nasdjli".as_bytes();
    //     assert_eq!(check_mutipart_header2(b), check_mutipart_header(b))
    // }
}
