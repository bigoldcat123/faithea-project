use std::fmt::Display;

use crate::response::HttpResponseModifier;

#[derive(Debug)]
pub enum Error {}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
impl std::error::Error for Error {}
impl HttpResponseModifier for Error {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut crate::response::HttpResponse,
    ) -> std::pin::Pin<
        Box<
            dyn Future<Output = Result<(), crate::handler::types::HttpHandlerError>>
                + 'a
                + Send
                + Sync,
        >,
    > {
        Box::pin(async move { Ok(()) })
    }
}
