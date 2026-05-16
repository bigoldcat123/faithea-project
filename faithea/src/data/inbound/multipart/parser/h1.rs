use bytes::{Buf, Bytes, BytesMut};
use tokio::io::AsyncWriteExt;

use crate::{
    data::inbound::multipart::{
        MultiPartFile, MultipartDataMap, Part,
        parser::{HeaderInfo, MultiPartBodyParserState, build_boundary_next_array},
    },
    map_str,
    request::RequestBody,
    server::BytesSource,
};
pub struct MultiPartBodyParser<'a, R> {
    r: R,
    buf: &'a mut BytesMut,
    boundary: &'a [u8],
    boundary_with_prefix: Vec<u8>,
    boundary_with_prefix_next: Vec<usize>,
    map: Option<MultipartDataMap>,
    state: MultiPartBodyParserState,
    header_info: HeaderInfo,
}
impl<'a, R: BytesSource> MultiPartBodyParser<'a, R> {
    fn new_with_start_state(r: R, buf: &'a mut BytesMut, boundary: &'a [u8]) -> Self {
        let map = Some(MultipartDataMap::new());
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
            boundary,
            boundary_with_prefix,
            boundary_with_prefix_next,
            state,
            map,
            header_info,
        }
    }
    async fn process(&mut self) -> Result<RequestBody, String> {
        use MultiPartBodyParserState::*;
        loop {
            match self.current_sate() {
                Start => {
                    self.remove_mutipart_body_prefix().await?;
                }
                Header => {
                    self.parse_header().await?;
                }
                Body => {
                    self.parse_body().await?;
                }
                End => return Ok(RequestBody::MultiPart(self.generate_multipart())),
            }
        }
    }
    pub async fn parse(
        r: R,
        buf: &'a mut BytesMut,
        boundary: &'a str,
    ) -> Result<RequestBody, String> {
        let mut state_machine = Self::new_with_start_state(r, buf, boundary.as_bytes());
        state_machine.process().await
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
                let _read_len = self.r.read_buf2(self.buf).await.map_err(map_str!())?;
                // if read_len == 0 {
                //     return Err("Unexpected EOF".to_string());
                // }
            }
            // let (is_ended, len) = check_body_end(self.buf,&self.boundary_with_prefix,&self.boundary_with_prefix_next);
            let (is_ended, len) = self.check_body_end();

            if is_ended {
                let mut b = self.buf.split_to(len);
                f.write_buf(&mut b).await.map_err(map_str!())?;
                let _ = self.buf.split_to(self.boundary.len() + 2 + 2 + 2);
                break;
            } else {
                let mut b = self.buf.split_to(self.buf.len() - self.boundary.len() - 6);
                f.write_buf(&mut b).await.map_err(map_str!())?;
            }
        }
        //delete file
        Ok(multipart_file)
    }

    fn check_body_end(&self) -> (bool, usize) {
        let mut i = 0;
        let mut j = 0;
        while i < self.buf.len() && j < self.boundary_with_prefix.len() {
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
                let _read_len = self.r.read_buf2(self.buf).await.map_err(map_str!())?;
                // if read_len == 0 {
                //     return Err("Unexpected EOF".to_string());
                // }
            }
            // println!("-> {:?} {:?} {:?} {:?}", buf,name,file_name,mime_type);
            // let (is_ended, len) = check_body_end(self.buf,&self.boundary_with_prefix,&self.boundary_with_prefix_next);
            let (is_ended, len) = self.check_body_end();
            if is_ended {
                let b = self.buf.split_to(len);
                simple_body.extend_from_slice(&b[..]);
                // /r/n --boundary /r/n => 2 + 2 + 2 + len
                let _ = self.buf.split_to(self.boundary.len() + 2 + 2 + 2);
                break;
            } else {
                let b = self.buf.split_to(self.buf.len() - self.boundary.len() - 6);
                simple_body.extend_from_slice(&b[..]);
            }
        }
        Ok(simple_body.freeze())
    }

    fn body_ends(&self) -> bool {
        &self.buf[..] == b"\r\n" && self.r.is_end()
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
    fn current_sate(&self) -> MultiPartBodyParserState {
        self.state
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
        }
        Ok(())
    }
    fn generate_multipart(&mut self) -> MultipartDataMap {
        self.map.take().unwrap()
    }
    async fn parse_header(&mut self) -> Result<(), String> {
        let header_len;
        loop {
            let (inner_is_ok, inner_header_len) = self.check_mutipart_header();
            if inner_is_ok {
                header_len = inner_header_len;
                break;
            }
            let read_len = self.r.read_buf2(self.buf).await.map_err(map_str!())?;
            if read_len == 0 {
                self.state = MultiPartBodyParserState::End;
                return Ok(());
            }
        }
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
    async fn remove_mutipart_body_prefix(&mut self) -> Result<(), String> {
        // remove pre_fix
        while self.buf.len() < self.boundary.len() + 2 {
            let _read_len = self.r.read_buf2(self.buf).await.map_err(map_str!())?;
            // if read_len == 0 {
            //     return Err("Unexpected EOF".to_string());
            // }
        }
        if &self.buf[..2] != b"--"
            || &self.buf[2..2 + self.boundary.len()] != self.boundary
            || &self.buf[2 + self.boundary.len()..2 + self.boundary.len() + 2] != b"\r\n"
        {
            return Err("Invalid boundary".to_string());
        }
        self.buf.advance(2 + self.boundary.len() + 2);
        self.state = MultiPartBodyParserState::Header;
        Ok(())
    }
}
