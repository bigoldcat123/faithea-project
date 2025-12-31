use std::{collections::HashMap, sync::LazyLock};

use bytes::Bytes;
use faithea::{request::HttpRequest, websocket::{data::WebSocketDataPayLoad, socket::WebSocket}};
use serde::{Deserialize, Serialize};
use tokio::sync::{
    Mutex,
    mpsc::Sender,
};

static WS_SENDERS: LazyLock<Mutex<HashMap<String, Sender<WebSocketDataPayLoad>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize, Deserialize)]
struct WsDataMessage {
    r#type: String,
    to: String,
    from: String,
    content: String,
}

pub async fn ws(
    websocket: WebSocket,
    req: HttpRequest,
) {
    let  (mut r,s) = websocket.split();
    let name = req.get_pathparam("name").unwrap();
    {
        let mut map = WS_SENDERS.lock().await;
        map.insert(name.clone(), s.clone());
    }
    while let Some(msg) = r.recv().await {
        let data = serde_json::from_slice::<WsDataMessage>(msg.as_bytes()).unwrap();
        let map = WS_SENDERS.lock().await;
        if let Some(sender) = map.get(&data.to) {
            let a:Bytes = serde_json::to_vec(&data).unwrap().into();
            sender.send(WebSocketDataPayLoad::text(a)).await.unwrap();
        }
    }
}
