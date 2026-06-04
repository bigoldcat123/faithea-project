use http::header::{CACHE_CONTROL, CONTENT_TYPE};

use crate::response::{HttpResponseModifier, HttpResponseModifierFuture};

pub struct SSE;
impl HttpResponseModifier for SSE {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut super::HttpResponse,
    ) -> HttpResponseModifierFuture<'a> {
        Box::pin(async move {
            res.add_header(CONTENT_TYPE, "text/event-stream".try_into()?);
            res.add_header(CACHE_CONTROL, "no-cache".try_into()?);
            Ok(())
        })
    }
}
