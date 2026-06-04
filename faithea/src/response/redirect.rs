use http::{HeaderValue, StatusCode, header::LOCATION};

use crate::response::{HttpResponseModifier, HttpResponseModifierFuture};

pub struct Redirect<P: AsRef<str>>(pub P);

impl<P: AsRef<str>> HttpResponseModifier for Redirect<P> {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut super::HttpResponse,
    ) -> HttpResponseModifierFuture<'a> {
        let p = self.0.as_ref().to_string();
        Box::pin(async move {
            res.add_header(LOCATION, HeaderValue::try_from(p)?);
            *res._innser.status_mut() = StatusCode::PERMANENT_REDIRECT;
            Ok(())
        })
    }
}
