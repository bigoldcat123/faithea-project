# Naming cleanup (unreleased)

This source revision standardizes spelling and makes asynchronous types and routing data easier to identify. These are **breaking Rust API renames**: update downstream imports, trait implementations, method calls, field accesses, and enum matches together. Legacy names are not retained as aliases. Crate/package names, HTTP routes, query keys, multipart field names, and serialized JSON fields are unchanged.

## Routing and requests

| Previous name | Current name |
| --- | --- |
| `handler::HandlerTire` | `handler::HandlerTrie` |
| `guard::GuardTire` | `guard::GuardTrie` |
| `Route.r` | `Route.components` |
| `RouteComponent::SingleSegWildCard` | `RouteComponent::SingleSegmentWildcard` |
| `RouteComponent::MultiSegWildCard` | `RouteComponent::MultiSegmentWildcard` |
| `Handler::WbeSocket` | `Handler::WebSocket` |
| `HandlerTire::websoekct_h1` | `HandlerTrie::websocket_h1` |
| `HandlerTire::websoekct_h2` | `HandlerTrie::websocket_h2` |
| `HttpRequest::get_pathparam` | `HttpRequest::get_path_param` |
| `regulate_url_path` | `normalize_url_path` |

## Handler futures and server services

`Result` now denotes a completed result; `Future` denotes asynchronous work producing a result. The old private boxed future alias `HttpHandlerResult` is now `BoxedHttpHandlerFuture`, and the public `HttpHandlerResult` names `Result<HttpResponse, HttpHandlerError>`.

| Previous name | Current name |
| --- | --- |
| `HttpHandlerResultOutput` | `HttpHandlerResult` |
| `HttpHandlerResultTrait` | `HttpHandlerFuture` |
| `HttphandlerErrorTrait` | `HttpHandlerErrorModifier` |
| `WebSocketHandlerResultTrait` | `WebSocketHandlerFuture` |
| `WebSocketHandlerTarit` | `WebSocketHandlerTrait` |
| `RawWebSocketHandlerTarit` | `RawWebSocketHandlerTrait` |
| `GuardResultTrait` | `GuardFuture` |
| `GlobaleHandlerResponseRaw` | `GlobalErrorHandlerFuture` |
| `GlobaleHandlerRaw` | `RawGlobalErrorHandler` |
| `HttpServerBuilder::globale_error_handler` | `HttpServerBuilder::global_error_handler` |
| `ServerFuncProvider` | `ServerContext` |
| `MyServiceFn` | `ServerService` |
| `my_service_fn` | `server_service` |
| `HyperIncommingBytesSource` | `HyperIncomingBytesSource` |

## Multipart, WebSocket, and shared I/O

| Previous name | Current name |
| --- | --- |
| `MultiPartFile` | `MultipartFile` |
| `MultiPartBodyParser` | `MultipartBodyParser` |
| `RequestBody::MultiPart` | `RequestBody::Multipart` |
| `WebSocketDataPayLoad` | `WebSocketPayload` |
| `WebSocketIncommingMessageParser` | `WebSocketIncomingMessageParser` |
| `WebSocketDataPayLoad.r#type` | `WebSocketPayload.message_type` |
| `faithea_websocket::Handle` | `faithea_websocket::WebSocketHandle` |
| `Handle.outcomming_message_receiver` | `WebSocketHandle.outgoing_message_receiver` |
| `BytesSource::read_buf2` | `BytesSource::read_bytes` |
| `BytesSink::write_all2` | `BytesSink::write_bytes` |
| `BytesSink::flush2` | `BytesSink::flush_bytes` |

`WebSocketIncomingMessageParser` was already available as an alias; it is now the actual struct name. Shared I/O signatures, buffer-append behavior, write-all behavior, flushing, and EOF handling are unchanged.

## Internal naming conventions

- Use `snake_case` for functions, fields, and local variables; use `PascalCase` for types and generic parameters.
- Spell `Multipart`, `Payload`, `Incoming`, `Outgoing`, `Receiver`, `Trie`, and `Trait` consistently.
- Use descriptive names such as `components`, `children`, `source`, `context`, and `prefix` for routing and transport state.
- Reserve a leading underscore for intentionally unused bindings or fields. The macro-injected `_req` remains available to existing handlers.

Private helper names and fields were also corrected, including multipart parser state, request parameter assembly, MIME extension lookup, and macro wrapper generation. Earlier version-specific blog examples remain historical; current READMEs and bilingual website examples use the new names.
