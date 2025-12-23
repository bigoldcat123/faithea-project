use crate::{data::inbound::multipart::MultipartDataMap, request::RequestBody};

pub mod h1;
pub mod h2;

#[derive(Debug,Clone, Copy)]
pub(crate) enum MultiPartBodyParserState {
    Start,
    Header,
    Body,
    End,
}
#[derive(Default, Debug)]
pub(crate) struct HeaderInfo {
    name: Option<String>,
    mime_type: Option<String>,
    file_name: Option<String>,
}

pub(crate) trait MultipartParserStateMachine {
    fn current_sate(&self) -> MultiPartBodyParserState;
    async fn remove_mutipart_body_prefix(&mut self) -> Result<(), String>;
    async fn parse_header(&mut self) -> Result<(), String>;
    async fn parse_body(&mut self) -> Result<(), String>;
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
    fn generate_multipart(&mut self) -> MultipartDataMap;
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
