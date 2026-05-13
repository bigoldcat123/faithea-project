use bytes::Bytes;
use tokio::sync::mpsc::Receiver;

use crate::response::{HttpResponseModifier, ResponseBody};

pub struct Stream {
    receiver: Option<Receiver<Bytes>>,
}

impl Stream {
    pub fn new(receiver: Receiver<Bytes>) -> Self {
        Self {
            receiver: Some(receiver),
        }
    }
}
impl HttpResponseModifier for Stream {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut super::HttpResponse,
    ) -> std::pin::Pin<
        Box<
            dyn Future<Output = Result<(), crate::handler::types::HttpHandlerError>>
                + 'a
                + Send
                + Sync,
        >,
    > {
        Box::pin(async move {
            let receiver = self.receiver.take().expect("must have this");
            res.set_body(ResponseBody::Stream(receiver));
            Ok(())
        })
    }
}
