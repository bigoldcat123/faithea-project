use std::{collections::HashMap, sync::LazyLock};

use faithea::{
    request::HttpRequest,
    websocket::{data::WebSocketPayload, socket::WebSocket},
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc::Sender};

static WS_SENDERS: LazyLock<Mutex<HashMap<String, Sender<WebSocketPayload>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize, Deserialize)]
struct WsDataMessage {
    #[serde(rename = "type")]
    message_type: String,
    to: String,
    from: String,
    content: String,
}

pub async fn ws(websocket: WebSocket, req: HttpRequest) {
    let (mut r, s) = websocket.split();
    let name = req.get_path_param("name").unwrap();
    {
        let mut map = WS_SENDERS.lock().await;
        map.insert(name.clone(), s.clone());
    }
    while let Some(msg) = r.recv().await {
        if let Ok(data) = serde_json::from_slice::<WsDataMessage>(msg.as_bytes()) {
            let map = WS_SENDERS.lock().await;
            if let Some(sender) = map.get(&data.to) {
                let a: String = serde_json::to_string(&data).unwrap();
                sender.send(WebSocketPayload::text(a)).await.unwrap();
            }
        } else {
            log::error!("parsing json error");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WsDataMessage;
    use serde_json::json;

    #[test]
    fn browser_chat_message_preserves_json_protocol() {
        let browser_message = json!({
            "type": "message",
            "to": "Bob",
            "from": "Alice",
            "content": "你好",
        });

        let message: WsDataMessage =
            serde_json::from_slice(&serde_json::to_vec(&browser_message).unwrap()).unwrap();
        assert_eq!(message.message_type, "message");

        let forwarded_message = serde_json::to_string(&message).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&forwarded_message).unwrap(),
            browser_message,
        );
    }
}
