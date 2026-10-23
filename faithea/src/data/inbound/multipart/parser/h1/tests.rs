use std::{collections::VecDeque, future::Future, pin::Pin};

use super::*;

const BOUNDARY: &str = "test-boundary";

struct ChunkedSource {
    chunks: VecDeque<Vec<u8>>,
    eof_reads: usize,
    signal_end: bool,
    fail_at_eof: bool,
    stall_at_eof: bool,
}

impl ChunkedSource {
    fn new(chunks: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self {
            chunks: chunks
                .into_iter()
                .filter(|chunk| !chunk.is_empty())
                .collect(),
            eof_reads: 0,
            signal_end: false,
            fail_at_eof: false,
            stall_at_eof: false,
        }
    }
}

impl BytesSource for ChunkedSource {
    fn read_bytes<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(chunk) = self.chunks.pop_front() {
                buf.extend_from_slice(&chunk);
                return Ok(chunk.len());
            }
            if self.stall_at_eof {
                return std::future::pending().await;
            }
            self.eof_reads += 1;
            // Bound erroneous EOF loops so a regression fails instead of hanging.
            if self.fail_at_eof || self.eof_reads > 1 {
                return Err("unexpected read after EOF".into());
            }
            Ok(0)
        })
    }

    fn is_end(&self) -> bool {
        self.signal_end && self.chunks.is_empty()
    }
}

fn text_body(value: &str) -> Vec<u8> {
    format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"text\"\r\n\r\n{value}\r\n--{BOUNDARY}--"
    )
    .into_bytes()
}

fn file_body() -> Vec<u8> {
    file_body_with_content(b"file data")
}

fn file_body_with_content(content: &[u8]) -> Vec<u8> {
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"upload\"; filename=\"sample.bin\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--").as_bytes());
    body
}

async fn parse(source: ChunkedSource) -> Result<MultipartDataMap, BodyParseError> {
    let mut buf = BytesMut::new();
    match MultipartBodyParser::parse(source, &mut buf, BOUNDARY).await? {
        RequestBody::Multipart(map) => Ok(map),
        body => panic!("expected multipart body, got {body:?}"),
    }
}

fn assert_text(map: &MultipartDataMap, key: &str, expected: &str) {
    let parts = map.get(key).expect("missing text field");
    assert_eq!(parts.len(), 1);
    assert!(matches!(&parts[0], Part::Lit(value) if value == expected));
}

#[tokio::test]
async fn malformed_disposition_parameters_return_errors() {
    for value in ["", "\"", "\"unfinished", "a\"", "中", "\"a\"junk"] {
        for parameter in [
            format!("name={value}"),
            format!("name=\"upload\"; filename={value}"),
        ] {
            let body = format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; {parameter}\r\n\r\npayload\r\n--{BOUNDARY}--"
            );
            for chunk_size in [1, body.len()] {
                let result = parse(ChunkedSource::new(
                    body.as_bytes()
                        .chunks(chunk_size)
                        .map(|chunk| chunk.to_vec()),
                ))
                .await;
                assert!(
                    matches!(result, Err(BodyParseError::Other(_))),
                    "parameter {parameter:?}, chunk_size={chunk_size}: {result:?}"
                );
            }
        }
    }
}

#[tokio::test]
async fn valid_disposition_values_are_not_truncated() {
    for (value, expected) in [
        ("a", "a"),
        ("abc", "abc"),
        ("\"\"", ""),
        ("\"中文😀\"", "中文😀"),
        ("\"a;b\"", "a;b"),
        ("\"a\\\"b\"", "a\"b"),
    ] {
        let body = format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name={value}\r\n\r\npayload\r\n--{BOUNDARY}--"
        );
        let map = parse(ChunkedSource::new(body.bytes().map(|byte| vec![byte])))
            .await
            .unwrap();
        assert_text(&map, expected, "payload");
    }
}

#[tokio::test]
async fn valid_filenames_are_preserved() {
    for (value, expected) in [
        ("a", "a"),
        ("\"\"", ""),
        ("\"中文😀;文档.txt\"", "中文😀;文档.txt"),
        ("\"a\\\"b.txt\"", "a\"b.txt"),
    ] {
        let body = format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"upload\"; filename={value}\r\n\r\npayload\r\n--{BOUNDARY}--"
        );
        let map = parse(ChunkedSource::new(body.bytes().map(|byte| vec![byte])))
            .await
            .unwrap();
        let Part::File(file) = &map["upload"][0] else {
            panic!("expected a file part");
        };
        assert_eq!(file.file_name.as_deref(), Some(expected));
        assert_eq!(std::fs::read(&file.temp_path).unwrap(), b"payload");
    }
}

#[tokio::test]
async fn truncated_text_request_returns_unexpected_eof_at_every_offset() {
    let body = text_body("hello");
    for signal_end in [false, true] {
        for end in 0..body.len() {
            let mut source = ChunkedSource::new([body[..end].to_vec()]);
            source.signal_end = signal_end;
            let result = parse(source).await;
            assert!(
                matches!(result, Err(BodyParseError::UnexpectedEof)),
                "truncated at {end}, signal_end={signal_end}: {result:?}"
            );
        }
    }
}

#[tokio::test]
async fn truncated_file_request_returns_unexpected_eof_at_every_offset() {
    let body = file_body();
    for signal_end in [false, true] {
        for end in 0..body.len() {
            let mut source = ChunkedSource::new([body[..end].to_vec()]);
            source.signal_end = signal_end;
            let result = parse(source).await;
            assert!(
                matches!(result, Err(BodyParseError::UnexpectedEof)),
                "file truncated at {end}, signal_end={signal_end}: {result:?}"
            );
        }
    }
}

#[tokio::test]
async fn opening_boundary_can_arrive_before_its_crlf() {
    let body = text_body("hello");
    let split = BOUNDARY.len() + 2;
    let map = parse(ChunkedSource::new([
        body[..split].to_vec(),
        body[split..].to_vec(),
    ]))
    .await
    .unwrap();
    assert_text(&map, "text", "hello");
}

#[tokio::test]
async fn text_request_parses_at_every_chunk_split() {
    let body = text_body("hello 世界");
    for split in 0..=body.len() {
        let map = parse(ChunkedSource::new([
            body[..split].to_vec(),
            body[split..].to_vec(),
        ]))
        .await
        .unwrap_or_else(|err| panic!("split at {split}: {err}"));
        assert_text(&map, "text", "hello 世界");
    }
}

#[tokio::test]
async fn file_request_parses_at_every_chunk_split() {
    let body = file_body();
    for split in 0..=body.len() {
        let map = parse(ChunkedSource::new([
            body[..split].to_vec(),
            body[split..].to_vec(),
        ]))
        .await
        .unwrap_or_else(|err| panic!("file split at {split}: {err}"));
        let parts = map.get("upload").unwrap();
        assert_eq!(parts.len(), 1);
        let Part::File(file) = &parts[0] else {
            panic!("expected a file part");
        };
        assert_eq!(file.file_name.as_deref(), Some("sample.bin"));
        assert_eq!(file.mime_type.as_deref(), Some("application/octet-stream"));
        assert_eq!(std::fs::read(&file.temp_path).unwrap(), b"file data");
        let path = std::path::PathBuf::from(&file.temp_path);
        drop(map);
        assert!(!path.exists(), "file leaked after multipart was dropped");
    }
}

#[tokio::test]
async fn uploaded_files_remain_readable_and_are_removed_on_drop() {
    let large_content: Vec<u8> = (0..3 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    for content in [Vec::new(), b"hello\0\xffupload".to_vec(), large_content] {
        let body = file_body_with_content(&content);
        for chunk_size in [body.len(), 4093] {
            let map = parse(ChunkedSource::new(
                body.chunks(chunk_size).map(|chunk| chunk.to_vec()),
            ))
            .await
            .unwrap();
            let Part::File(file) = &map["upload"][0] else {
                panic!("expected a file part");
            };
            let path = std::path::PathBuf::from(&file.temp_path);
            assert_eq!(std::fs::read(&path).unwrap(), content);
            drop(map);
            assert!(!path.exists(), "file leaked after multipart was dropped");
        }
    }
}

fn file_followed_by_truncated_header() -> Vec<u8> {
    let mut body = file_body();
    body.truncate(body.len() - 2);
    body.extend_from_slice(b"\r\nContent-Disposition: form-data;");
    body
}

#[tokio::test]
async fn uploaded_files_are_removed_if_a_later_part_is_truncated() {
    let source = ChunkedSource::new([file_followed_by_truncated_header()]);
    let mut buf = BytesMut::new();
    let mut parser =
        MultipartBodyParser::new_with_start_state(source, &mut buf, BOUNDARY.as_bytes());
    parser.remove_multipart_body_prefix().await.unwrap();
    parser.parse_header().await.unwrap();
    parser.parse_body().await.unwrap();
    let Part::File(file) = &parser.map.as_ref().unwrap()["upload"][0] else {
        panic!("expected a file part");
    };
    let path = std::path::PathBuf::from(&file.temp_path);
    assert!(path.exists());

    assert!(matches!(
        parser.process().await,
        Err(BodyParseError::UnexpectedEof)
    ));
    drop(parser);
    assert!(!path.exists(), "file leaked after parsing failed");
}

#[tokio::test]
async fn uploaded_files_are_removed_when_parsing_is_cancelled() {
    let mut source = ChunkedSource::new([file_followed_by_truncated_header()]);
    source.stall_at_eof = true;
    let mut buf = BytesMut::new();
    let mut parser =
        MultipartBodyParser::new_with_start_state(source, &mut buf, BOUNDARY.as_bytes());
    parser.remove_multipart_body_prefix().await.unwrap();
    parser.parse_header().await.unwrap();
    parser.parse_body().await.unwrap();
    let Part::File(file) = &parser.map.as_ref().unwrap()["upload"][0] else {
        panic!("expected a file part");
    };
    let path = std::path::PathBuf::from(&file.temp_path);
    assert!(path.exists());

    let mut parsing = Box::pin(async move { parser.process().await });
    std::future::poll_fn(|cx| {
        assert!(parsing.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    drop(parsing);
    assert!(!path.exists(), "file leaked after parsing was cancelled");
}

#[tokio::test]
async fn multiple_parts_and_empty_values_parse_one_byte_at_a_time() {
    let body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"empty\"\r\n\r\n\r\n--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"text\"\r\n\r\nhello\r\n--{BOUNDARY}--\r\n"
    );
    let map = parse(ChunkedSource::new(body.bytes().map(|byte| vec![byte])))
        .await
        .unwrap();
    assert_eq!(map.len(), 2);
    assert_text(&map, "empty", "");
    assert_text(&map, "text", "hello");
}

#[tokio::test]
async fn closing_boundary_finishes_without_reading_transport_eof() {
    for suffix in ["", "\r\n", "\r\nepilogue"] {
        let mut body = text_body("");
        body.extend_from_slice(suffix.as_bytes());
        let mut source = ChunkedSource::new([body]);
        source.fail_at_eof = true;
        let map = parse(source).await.unwrap();
        assert_text(&map, "text", "");
    }
}

#[tokio::test]
async fn empty_multipart_finishes_at_closing_boundary() {
    let body = format!("--{BOUNDARY}--");
    let map = parse(ChunkedSource::new(body.bytes().map(|byte| vec![byte])))
        .await
        .unwrap();
    assert!(map.is_empty());
}

#[tokio::test]
async fn boundary_like_payload_does_not_hide_the_real_delimiter() {
    let value = format!("before\r\n--{BOUNDARY}XYafter\r\n--{BOUNDARY}Z");
    let body = text_body(&value);
    for split in 0..=body.len() {
        let map = parse(ChunkedSource::new([
            body[..split].to_vec(),
            body[split..].to_vec(),
        ]))
        .await
        .unwrap_or_else(|err| panic!("boundary-like payload split at {split}: {err}"));
        assert_text(&map, "text", &value);
    }
}

#[tokio::test]
async fn invalid_opening_boundary_returns_invalid_boundary() {
    for body in [
        format!("XX{BOUNDARY}\r\n"),
        "--wrong-boundary\r\n".to_string(),
        format!("--{BOUNDARY}XY"),
    ] {
        assert!(matches!(
            parse(ChunkedSource::new([body.into_bytes()])).await,
            Err(BodyParseError::InvalidBoundary)
        ));
    }
}

#[tokio::test]
async fn transport_read_errors_are_preserved() {
    let mut source = ChunkedSource::new([]);
    source.fail_at_eof = true;
    assert!(matches!(
        parse(source).await,
        Err(BodyParseError::Other(message)) if message == "unexpected read after EOF"
    ));
}
