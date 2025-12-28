use http::{
    HeaderMap,
    header::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    },
};

use crate::response::HttpResponseModifier;

pub struct CORS;

impl HttpResponseModifier for CORS {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut super::HttpResponse,
    ) -> std::pin::Pin<
        Box<dyn Future<Output = Result<(), crate::handler::HttpHandlerError>> + 'a + Send + Sync>,
    > {
        Box::pin(async move {
            let mut header = HeaderMap::new();
            header.insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
            header.insert(ACCESS_CONTROL_ALLOW_HEADERS, "*".parse().unwrap());
            header.insert(
                ACCESS_CONTROL_ALLOW_METHODS,
                "GET, POST, PUT, DELETE".parse().unwrap(),
            );
            header.insert(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true".parse().unwrap());
            header.modify(res).await?;
            Ok(())
        })
    }
}
