// use std::collections::HashSet;

// use bytes::Bytes;
// use http::{
//     HeaderMap, HeaderName, HeaderValue, Method, StatusCode,
//     header::{
//         CONNECTION, CONTENT_LENGTH, CONTENT_TYPE, HOST, PROXY_AUTHENTICATE, PROXY_AUTHORIZATION,
//         TE, TRAILER, TRANSFER_ENCODING, UPGRADE,
//     },
// };
// use reqwest::{
//     Client, RequestBuilder, Url,
//     multipart::{Form, Part as ReqwestPart},
// };

// use crate::{
//     data::inbound::multipart::Part,
//     request::{HttpRequest, RequestBody},
//     response::{ HttpResponse, ResponseBody},
// };

// #[derive(Clone)]
// pub(crate) struct Proxy {
//     client: Client,
//     target: Url,
// }

// impl Proxy {
//     pub(crate) fn new(target: &str) -> Self {
//         let target = Url::parse(target).expect("proxy target must be an absolute http(s) URL");
//         assert!(
//             matches!(target.scheme(), "http" | "https"),
//             "proxy target must use http or https"
//         );
//         Self {
//             client: Client::new(),
//             target,
//         }
//     }

//     pub(crate) async fn forward(&self, req: HttpRequest) -> HttpResponse {
//         match self.try_forward(req).await {
//             Ok(response) => response,
//             Err(error) => {
//                 log::error!("proxy request failed: {error}");
//                 bad_gateway(error.to_string())
//             }
//         }
//     }

//     async fn try_forward(
//         &self,
//         req: HttpRequest,
//     ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
//         let remainder = req.multi_seg_param.as_deref();
//         let target = self.target_url(remainder, req.uri().query());
//         let (parts, body) = req._inner.into_parts();
//         let is_multipart = matches!(body, Some(RequestBody::MultiPart(_)));
//         let method = parts.method.clone();

//         let mut request = self.client.request(parts.method, target);
//         request = copy_request_headers(request, &parts.headers, is_multipart);
//         request = set_request_body(request, body).await?;

//         let upstream = request.send().await?;
//         let status = upstream.status();
//         let headers = upstream.headers().clone();
//         let body = upstream.bytes().await?;

//         Ok(upstream_response(method, status, headers, body))
//     }

//     fn target_url(&self, remainder: Option<&str>, query: Option<&str>) -> Url {
//         let mut target = self.target.clone();
//         if let Some(remainder) = remainder.filter(|value| !value.is_empty()) {
//             let path = format!(
//                 "{}/{}",
//                 target.path().trim_end_matches('/'),
//                 remainder.trim_start_matches('/')
//             );
//             target.set_path(&path);
//         }
//         if query.is_some() {
//             target.set_query(query);
//         }
//         target
//     }
// }

// fn copy_request_headers(
//     mut request: RequestBuilder,
//     headers: &HeaderMap,
//     is_multipart: bool,
// ) -> RequestBuilder {
//     let connection_headers = connection_header_names(headers);
//     for (name, value) in headers {
//         if is_hop_by_hop(name)
//             || connection_headers.contains(name)
//             || *name == HOST
//             || *name == CONTENT_LENGTH
//             || name.as_str() == "x-forwarded-host"
//             || (is_multipart && *name == CONTENT_TYPE)
//         {
//             continue;
//         }
//         request = request.header(name, value);
//     }

//     if let Some(host) = headers.get(HOST) {
//         request = request.header(HeaderName::from_static("x-forwarded-host"), host);
//     }
//     request
// }

// // async fn set_request_body(
// //     request: RequestBuilder,
// //     body: Option<RequestBody>,
// // ) -> Result<RequestBuilder, Box<dyn std::error::Error + Send + Sync>> {
// //     match body {
// //         Some(RequestBody::Simple(body)) => Ok(request.body(body)),
// //         Some(RequestBody::Stream(path)) => Ok(request.body(tokio::fs::read(path).await?)),
// //         Some(RequestBody::MultiPart(fields)) => {
// //             let mut form = Form::new();
// //             for (name, parts) in fields {
// //                 for part in parts {
// //                     form = match part {
// //                         Part::Lit(value) => form.text(name.clone(), value),
// //                         Part::File(file) => {
// //                             let mut upload =
// //                                 ReqwestPart::bytes(tokio::fs::read(&file.temp_path).await?);
// //                             if let Some(file_name) = &file.file_name {
// //                                 upload = upload.file_name(file_name.clone());
// //                             }
// //                             if let Some(mime_type) = &file.mime_type {
// //                                 upload = upload.mime_str(mime_type)?;
// //                             }
// //                             form.part(name.clone(), upload)
// //                         }
// //                     };
// //                 }
// //             }
// //             Ok(request.multipart(form))
// //         }
// //         Some(RequestBody::WebSocketStreamBodyHttp1(_))
// //         | Some(RequestBody::WebSocketStreamBodyHttp2(_)) => {
// //             Err("websocket requests cannot be forwarded by proxy".into())
// //         }
// //         None => Ok(request),
// //     }
// // }

// // fn upstream_response(
// //     method: Method,
// //     status: StatusCode,
// //     headers: HeaderMap,
// //     body: Bytes,
// // ) -> HttpResponse {
// //     let mut response = HttpResponse::new();
// //     *response._innser.status_mut() = status;
// //     let connection_headers = connection_header_names(&headers);
// //     for (name, value) in &headers {
// //         if !is_hop_by_hop(name) && !connection_headers.contains(name) {
// //             response
// //                 ._innser
// //                 .headers_mut()
// //                 .append(name.clone(), value.clone());
// //         }
// //     }
// //     if method != Method::HEAD
// //         && status != StatusCode::NO_CONTENT
// //         && status != StatusCode::NOT_MODIFIED
// //         && !response._innser.headers().contains_key(CONTENT_LENGTH)
// //     {
// //         response.add_header(
// //             CONTENT_LENGTH,
// //             HeaderValue::from_str(&body.len().to_string()).expect("body length is a valid header"),
// //         );
// //     }
// //     response.set_body(ResponseBody::Simple(body));
// //     response
// // }

// fn bad_gateway(message: String) -> HttpResponse {
//     let body = Bytes::from(message);
//     let mut response = HttpResponse::new();
//     *response._innser.status_mut() = StatusCode::BAD_GATEWAY;
//     response.add_header(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
//     response.add_header(
//         CONTENT_LENGTH,
//         HeaderValue::from_str(&body.len().to_string()).expect("body length is a valid header"),
//     );
//     response.set_body(ResponseBody::Simple(body));
//     response
// }

// fn is_hop_by_hop(name: &HeaderName) -> bool {
//     *name == CONNECTION
//         || name.as_str() == "keep-alive"
//         || name.as_str() == "proxy-connection"
//         || *name == PROXY_AUTHENTICATE
//         || *name == PROXY_AUTHORIZATION
//         || *name == TE
//         || *name == TRAILER
//         || *name == TRANSFER_ENCODING
//         || *name == UPGRADE
// }

// fn connection_header_names(headers: &HeaderMap) -> HashSet<HeaderName> {
//     headers
//         .get_all(CONNECTION)
//         .iter()
//         .filter_map(|value| value.to_str().ok())
//         .flat_map(|value| value.split(','))
//         .filter_map(|name| HeaderName::from_bytes(name.trim().as_bytes()).ok())
//         .collect()
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use http::Request;
//     use tokio::{
//         io::{AsyncReadExt, AsyncWriteExt},
//         net::TcpListener,
//     };

//     #[test]
//     fn appends_wildcard_path_and_query_to_target() {
//         let proxy = Proxy::new("http://localhost:7799/api/v1");
//         let target = proxy.target_url(Some("users/42"), Some("active=true"));

//         assert_eq!(
//             target.as_str(),
//             "http://localhost:7799/api/v1/users/42?active=true"
//         );
//     }

//     #[test]
//     fn preserves_target_query_when_request_has_no_query() {
//         let proxy = Proxy::new("http://localhost:7799/api/v1?token=server");
//         let target = proxy.target_url(Some("users"), None);

//         assert_eq!(
//             target.as_str(),
//             "http://localhost:7799/api/v1/users?token=server"
//         );
//     }

//     #[tokio::test(flavor = "current_thread")]
//     async fn forwards_request_and_maps_upstream_response() {
//         let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
//         let addr = listener.local_addr().unwrap();
//         let upstream = tokio::spawn(async move {
//             let (mut socket, _) = listener.accept().await.unwrap();
//             let mut request = Vec::new();
//             let mut chunk = [0_u8; 1024];
//             loop {
//                 let len = socket.read(&mut chunk).await.unwrap();
//                 request.extend_from_slice(&chunk[..len]);
//                 if request.ends_with(b"hello proxy") {
//                     break;
//                 }
//             }
//             socket
//                 .write_all(
//                     b"HTTP/1.1 201 Created\r\nContent-Length: 9\r\nX-Upstream: yes\r\nConnection: close\r\n\r\nforwarded",
//                 )
//                 .await
//                 .unwrap();
//             String::from_utf8(request).unwrap()
//         });

//         let mut req = HttpRequest::from_req(
//             Request::builder()
//                 .method(Method::POST)
//                 .uri("/api/users?active=true")
//                 .header(HOST, "localhost:8899")
//                 .body(Some(RequestBody::Simple(Bytes::from_static(
//                     b"hello proxy",
//                 ))))
//                 .unwrap(),
//         );
//         req.multi_seg_param = Some("users".to_string());

//         let response = Proxy::new(&format!("http://{addr}/api/v1"))
//             .forward(req)
//             .await;
//         let raw_request = upstream.await.unwrap();

//         assert!(raw_request.starts_with("POST /api/v1/users?active=true HTTP/1.1\r\n"));
//         assert!(raw_request.contains("x-forwarded-host: localhost:8899\r\n"));
//         assert_eq!(response._innser.status(), StatusCode::CREATED);
//         assert_eq!(response._innser.headers()["x-upstream"], "yes");
//         assert!(response._innser.headers().get(CONNECTION).is_none());
//         assert!(matches!(
//             response._innser.body(),
//             ResponseBody::Simple(body) if body == "forwarded"
//         ));
//     }
// }
