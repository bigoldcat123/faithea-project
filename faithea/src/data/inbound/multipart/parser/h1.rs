use bytes::{Buf, Bytes, BytesMut};
use tokio::io::AsyncWriteExt;

use crate::{
    data::inbound::multipart::{
        MultipartDataMap, MultipartFile, Part,
        parser::{HeaderInfo, MultipartBodyParserState, build_boundary_next_array, disposition},
    },
    error::BodyParseError,
    request::{BodyLimits, RequestBody},
    server::BytesSource,
};

#[cfg(test)]
mod tests;

pub struct MultipartBodyParser<'a, R> {
    source: R,
    buf: &'a mut BytesMut,
    boundary: &'a [u8],
    boundary_with_prefix: Vec<u8>,
    boundary_with_prefix_next: Vec<usize>,
    map: Option<MultipartDataMap>,
    state: MultipartBodyParserState,
    header_info: HeaderInfo,
    limits: BodyLimits,
    parts: usize,
}
impl<'a, R: BytesSource> MultipartBodyParser<'a, R> {
    fn new_with_start_state(source: R, buf: &'a mut BytesMut, boundary: &'a [u8]) -> Self {
        let map = Some(MultipartDataMap::new());
        let mut boundary_with_prefix = BytesMut::new();
        boundary_with_prefix.extend_from_slice(b"\r\n--");
        boundary_with_prefix.extend_from_slice(boundary);
        let boundary_with_prefix = boundary_with_prefix.to_vec();
        let boundary_with_prefix_next = build_boundary_next_array(&boundary_with_prefix);
        let state = MultipartBodyParserState::Start;
        let header_info = Default::default();
        Self {
            source,
            buf,
            boundary,
            boundary_with_prefix,
            boundary_with_prefix_next,
            state,
            map,
            header_info,
            limits: BodyLimits::default(),
            parts: 0,
        }
    }
    async fn process(&mut self) -> Result<RequestBody, BodyParseError> {
        use MultipartBodyParserState::*;
        loop {
            match self.current_state() {
                Start => {
                    self.remove_multipart_body_prefix().await?;
                }
                Header => {
                    self.parse_header().await?;
                }
                Body => {
                    self.parse_body().await?;
                }
                End => return Ok(RequestBody::Multipart(self.generate_multipart())),
            }
        }
    }
    pub async fn parse(
        source: R,
        buf: &'a mut BytesMut,
        boundary: &'a str,
    ) -> Result<RequestBody, BodyParseError> {
        Self::parse_with_limits(source, buf, boundary, BodyLimits::default()).await
    }

    pub(crate) async fn parse_with_limits(
        source: R,
        buf: &'a mut BytesMut,
        boundary: &'a str,
        limits: BodyLimits,
    ) -> Result<RequestBody, BodyParseError> {
        let mut state_machine = Self::new_with_start_state(source, buf, boundary.as_bytes());
        state_machine.limits = limits;
        state_machine.process().await
    }

    fn is_file_body(&self) -> bool {
        self.header_info.file_name.is_some() || self.header_info.mime_type.is_some()
    }

    async fn read_more(&mut self) -> Result<(), BodyParseError> {
        if self.source.is_end() || self.source.read_bytes(self.buf).await? == 0 {
            return Err(BodyParseError::UnexpectedEof);
        }
        Ok(())
    }

    async fn ensure_len(&mut self, len: usize) -> Result<(), BodyParseError> {
        while self.buf.len() < len {
            self.read_more().await?;
        }
        Ok(())
    }

    fn consume_body_boundary(&mut self) {
        self.buf.advance(self.boundary_with_prefix.len());
        self.state = if &self.buf[..2] == b"--" {
            MultipartBodyParserState::End
        } else {
            MultipartBodyParserState::Header
        };
        self.buf.advance(2);
    }

    async fn parse_file_body(&mut self) -> Result<MultipartFile, BodyParseError> {
        let file_name = self.header_info.file_name.take();
        let mime_type = self.header_info.mime_type.take();
        let temp_file = tempfile::Builder::new()
            .prefix("faithea-multipart-")
            .tempfile()?;
        let (file, temp_path) = temp_file.into_parts();
        let mut f = tokio::fs::File::from_std(file);
        let delimiter_len = self.boundary_with_prefix.len() + 2;
        let mut written = 0;
        loop {
            self.ensure_len(delimiter_len).await?;
            let (is_ended, len) = self.check_body_end();

            if is_ended {
                Self::check_size(written, len, self.limits.max_file_size, "multipart file")?;
                let mut b = self.buf.split_to(len);
                f.write_all_buf(&mut b).await?;
                self.consume_body_boundary();
                break;
            } else {
                // Retain a possible delimiter split across the next read.
                let len = self.buf.len() - (delimiter_len - 1);
                Self::check_size(written, len, self.limits.max_file_size, "multipart file")?;
                written += len;
                let mut b = self.buf.split_to(len);
                f.write_all_buf(&mut b).await?;
            }
        }
        f.flush().await?;
        // Transfer cleanup to MultipartFile only after every write has completed.
        let temp_path = temp_path.keep().map_err(|err| err.error)?;
        Ok(MultipartFile {
            temp_path: temp_path.to_string_lossy().into_owned(),
            file_name,
            mime_type,
        })
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
            if j == self.boundary_with_prefix.len() {
                if i + 1 < self.buf.len()
                    && (&self.buf[i..i + 2] == b"\r\n" || &self.buf[i..i + 2] == b"--")
                {
                    return (true, i - self.boundary_with_prefix.len());
                }
                // A boundary-like payload must not hide a later real delimiter.
                j = self.boundary_with_prefix_next[j - 1];
            }
        }
        (false, 0)
    }

    async fn parse_lit_body(&mut self) -> Result<Bytes, BodyParseError> {
        let mut simple_body = BytesMut::new();
        let delimiter_len = self.boundary_with_prefix.len() + 2;
        loop {
            self.ensure_len(delimiter_len).await?;
            let (is_ended, len) = self.check_body_end();
            if is_ended {
                Self::check_size(
                    simple_body.len(),
                    len,
                    self.limits.max_field_size,
                    "multipart field",
                )?;
                let b = self.buf.split_to(len);
                simple_body.extend_from_slice(&b[..]);
                self.consume_body_boundary();
                break;
            } else {
                let len = self.buf.len() - (delimiter_len - 1);
                Self::check_size(
                    simple_body.len(),
                    len,
                    self.limits.max_field_size,
                    "multipart field",
                )?;
                let b = self.buf.split_to(len);
                simple_body.extend_from_slice(&b[..]);
            }
        }
        Ok(simple_body.freeze())
    }

    fn check_size(
        current: usize,
        additional: usize,
        limit: usize,
        resource: &'static str,
    ) -> Result<(), BodyParseError> {
        if additional > limit.saturating_sub(current) {
            return Err(BodyParseError::LimitExceeded { resource, limit });
        }
        Ok(())
    }

    fn check_multipart_header(&self) -> (bool, usize) {
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

    fn process_multipart_header(&mut self, header_line: &str) -> Result<(), BodyParseError> {
        if let Some((k, v)) = header_line.split_once(":") {
            if k.eq_ignore_ascii_case("Content-Disposition") {
                let parameters = disposition::parameters(v)
                    .map_err(|message| BodyParseError::Other(message.to_string()))?;
                for (name, value) in parameters {
                    if name.eq_ignore_ascii_case("name") {
                        self.header_info.name = Some(value);
                    } else if name.eq_ignore_ascii_case("filename") {
                        self.header_info.file_name = Some(value);
                    }
                }
            } else if k.eq_ignore_ascii_case("Content-Type") {
                self.header_info.mime_type = Some(v.trim().to_string())
            }
        }
        Ok(())
    }
    fn current_state(&self) -> MultipartBodyParserState {
        self.state
    }
    async fn parse_body(&mut self) -> Result<(), BodyParseError> {
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
            let data = String::from_utf8(a.to_vec())?;
            if let Some(map) = self.map.as_mut() {
                map.entry(key_name).or_default().push(Part::Lit(data));
            }
        }
        Ok(())
    }
    fn generate_multipart(&mut self) -> MultipartDataMap {
        self.map.take().unwrap()
    }
    async fn parse_header(&mut self) -> Result<(), BodyParseError> {
        if self.parts >= self.limits.max_parts {
            return Err(BodyParseError::LimitExceeded {
                resource: "multipart part count",
                limit: self.limits.max_parts,
            });
        }
        self.parts += 1;
        let header_len;
        loop {
            let (inner_is_ok, inner_header_len) = self.check_multipart_header();
            if inner_is_ok {
                Self::check_size(
                    0,
                    inner_header_len,
                    self.limits.max_part_header_size,
                    "multipart part headers",
                )?;
                header_len = inner_header_len;
                break;
            }
            Self::check_size(
                0,
                self.buf.len(),
                self.limits.max_part_header_size,
                "multipart part headers",
            )?;
            self.read_more().await?;
        }
        let b = self.buf.split_to(header_len);
        let b = str::from_utf8(&b)?;
        for l in b.split("\r\n") {
            if !l.is_empty() {
                self.process_multipart_header(l)?;
            }
        }
        self.state = MultipartBodyParserState::Body;
        Ok(())
    }
    async fn remove_multipart_body_prefix(&mut self) -> Result<(), BodyParseError> {
        self.ensure_len(self.boundary.len() + 4).await?;
        if &self.buf[..2] != b"--" || &self.buf[2..2 + self.boundary.len()] != self.boundary {
            return Err(BodyParseError::InvalidBoundary);
        }
        self.state = match &self.buf[2 + self.boundary.len()..4 + self.boundary.len()] {
            b"\r\n" => MultipartBodyParserState::Header,
            b"--" => MultipartBodyParserState::End,
            _ => return Err(BodyParseError::InvalidBoundary),
        };
        self.buf.advance(self.boundary.len() + 4);
        Ok(())
    }
}
