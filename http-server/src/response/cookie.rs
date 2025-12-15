use std::collections::HashMap;

use crate::response::HttpResponseModifier;

#[derive(Debug, Default)]
pub struct Cookie {
    _innser: HashMap<String, String>,
}

impl HttpResponseModifier for Cookie {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut super::HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), String>> + 'a + Send + Sync>> {
        Box::pin(async move {
            res.headers.add("key".to_string(), "value".to_string());
            Ok(())
        })
    }
}
