# websocket

A small, transport-agnostic server-side WebSocket protocol layer.

This crate does not perform HTTP upgrade handshakes. It assumes your stream is already positioned at WebSocket frames, then handles the WebSocket framing rules for server code: client masking, text/binary messages, fragmentation, ping/pong, close frames, and basic protocol validation.

## Design

The library keeps a channel-oriented design:

- `BytesSource` reads raw bytes from any async transport.
- `WebSocketIncommingMessageParser::new(source, outgoing_tx)` starts a parser over that source.
- The returned `incoming_rx` receives parsed `WebSocketDataPayLoad` messages.
- Automatic `pong` and `close` replies are sent into the `outgoing_tx` you provide.
- `WebSocketDataPayLoad::serialize_to_socket(&mut sink)` writes server frames to any `BytesSink`.

The crate is intentionally unaware of TCP, TLS, HTTP, routing, or handshake policy.

## Quick Start

```rust
use tokio::sync::mpsc;
use websocket::{WebSocketDataPayLoad, WebSocketIncommingMessageParser};

# async fn run<S>(source: S)
# where
#     S: websocket::BytesSource + 'static,
# {
let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<WebSocketDataPayLoad>(32);
let app_outgoing_tx = outgoing_tx.clone();

let (parser, mut incoming_rx) =
    WebSocketIncommingMessageParser::new(source, outgoing_tx);
parser.start();

tokio::spawn(async move {
    while let Some(message) = incoming_rx.recv().await {
        let _ = app_outgoing_tx.send(message).await;
    }
});
# }
```

To write outgoing messages, consume your outgoing receiver and serialize each payload:

```rust
# async fn write_loop<W>(
#     mut writer: W,
#     mut outgoing_rx: tokio::sync::mpsc::Receiver<websocket::WebSocketDataPayLoad>,
# ) -> Result<(), String>
# where
#     W: websocket::BytesSink,
# {
while let Some(message) = outgoing_rx.recv().await {
    message.serialize_to_socket(&mut writer).await?;
}
# Ok(())
# }
```

## Examples

Run the in-memory round trip example:

```bash
cargo run --example in_memory_roundtrip
```

Run a TCP echo skeleton for streams that are already upgraded to WebSocket:

```bash
cargo run --example tcp_echo
```

## Supported Protocol Surface

- Server-side frame parsing.
- Client-to-server masked frames.
- Server-to-client unmasked frames.
- Payload lengths `0..125`, `126`, and `127`.
- Text and binary messages.
- Fragmented text/binary message assembly.
- Ping auto-pong.
- Close auto-reply.
- RSV, opcode, control-frame, frame-size, and message-size validation.

## Not Included

- HTTP upgrade handshake.
- Client-side WebSocket behavior.
- TLS.
- Extensions such as `permessage-deflate`.
- Subprotocol negotiation.

## Public Types

- `BytesSource`: async byte input abstraction.
- `BytesSink`: async byte output abstraction.
- `WebSocketIncommingMessageParser`: parser/driver for incoming server messages.
- `WebSocketIncomingMessageParser`: correctly-spelled alias.
- `WebSocketDataPayLoad`: text/binary/ping/pong/close payload.
- `WebSocketMessageType`: message opcode/type.
- `ProtocolError`: internal protocol validation error type.
