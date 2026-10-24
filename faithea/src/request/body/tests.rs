use std::collections::VecDeque;

use super::*;

struct Source {
    chunks: VecDeque<Vec<u8>>,
    stall: bool,
}

impl Source {
    fn new(chunks: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self {
            chunks: chunks.into_iter().collect(),
            stall: false,
        }
    }
}

impl BytesSource for Source {
    fn read_bytes<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(chunk) = self.chunks.pop_front() {
                buf.extend_from_slice(&chunk);
                return Ok(chunk.len());
            }
            if self.stall {
                return std::future::pending().await;
            }
            // Deliberately do not signal EOF via is_end().
            Ok(0)
        })
    }
    fn is_end(&self) -> bool {
        false
    }
}

#[tokio::test]
async fn actual_bytes_and_prefilled_buffers_are_counted_at_the_exact_limit() {
    let limits = BodyLimits {
        max_body_size: 4,
        ..BodyLimits::default()
    };
    for size in [4, 5] {
        let mut buf = BytesMut::from(&b"ab"[..]);
        let result = parse_body_frame_with_limits(
            Source::new((2..size).map(|_| vec![b'x'])),
            &mut buf,
            &HeaderMap::new(),
            limits,
        )
        .await;
        if size == 4 {
            assert!(matches!(result, Ok(RequestBody::Simple(bytes)) if bytes.as_ref() == b"abxx"));
        } else {
            assert!(matches!(
                result,
                Err(BodyParseError::LimitExceeded {
                    resource: "request body",
                    ..
                })
            ));
        }
    }
}

#[tokio::test]
async fn oversized_content_length_is_rejected_without_polling_the_source() {
    struct NeverRead;
    impl BytesSource for NeverRead {
        fn read_bytes<'a>(
            &'a mut self,
            _: &'a mut BytesMut,
        ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
            panic!("oversized Content-Length must be rejected before reading")
        }
        fn is_end(&self) -> bool {
            false
        }
    }
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_LENGTH, "5".parse().unwrap());
    let result = parse_body_frame_with_limits(
        NeverRead,
        &mut BytesMut::new(),
        &headers,
        BodyLimits {
            max_body_size: 4,
            ..BodyLimits::default()
        },
    )
    .await;
    assert!(matches!(result, Err(BodyParseError::LimitExceeded { .. })));
}

#[tokio::test]
async fn deadline_covers_simple_bodies_and_multipart_epilogues() {
    for multipart in [false, true] {
        let mut headers = HeaderMap::new();
        let mut source = Source::new([]);
        source.stall = true;
        if multipart {
            headers.insert(
                "content-type",
                "multipart/form-data; boundary=test".parse().unwrap(),
            );
            source.chunks.push_back(b"--test--\r\n".to_vec());
        }
        let result = parse_body_frame_with_limits(
            source,
            &mut BytesMut::new(),
            &headers,
            BodyLimits {
                read_timeout: Duration::from_millis(20),
                ..BodyLimits::default()
            },
        )
        .await;
        assert!(matches!(result, Err(BodyParseError::Timeout)), "{result:?}");
    }
}

#[tokio::test]
async fn multipart_epilogues_cannot_bypass_the_total_size_limit() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        "multipart/form-data; boundary=test".parse().unwrap(),
    );
    let result = parse_body_frame_with_limits(
        Source::new([b"--test--".to_vec(), b"epilogue".to_vec()]),
        &mut BytesMut::new(),
        &headers,
        BodyLimits {
            max_body_size: 8,
            ..BodyLimits::default()
        },
    )
    .await;
    assert!(matches!(
        result,
        Err(BodyParseError::LimitExceeded {
            resource: "request body",
            ..
        })
    ));
}
