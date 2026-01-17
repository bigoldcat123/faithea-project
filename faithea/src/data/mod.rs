use bytes::Buf;
use serde::{Deserialize, Serialize};

use crate::{handler::types::HttpHandlerError, request::{HttpRequest, RequestBody, TryFromRequest}};

pub mod inbound;
pub mod outbound;

#[derive(Serialize, Debug)]
pub struct Json<T>(pub T);
impl <'a,T:Deserialize<'a>> TryFromRequest<'a> for Json<T> {
    fn try_from_request(req: &'a mut HttpRequest) -> Result<Self, HttpHandlerError> {
        if let Some(RequestBody::Simple(body)) = req._inner.body() {
            Ok(Self(serde_json::from_slice::<T>(body.chunk())?))
        } else {
            Err(crate::error::Error::before_handler_empty_request_body())
        }
    }
}
impl<T> std::ops::Deref for Json<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> std::ops::DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
#[cfg(test)]
mod tests {
    use http::{HeaderMap, StatusCode};
    use serde::Deserialize;

    use crate::{
        TryConvertFrom, request::HttpRequest, res_modifiers, response::{HttpResponse, HttpResponseModifier, ResponseBody}
    };

    use super::*;
    #[derive(Deserialize, Serialize, Debug)]
    struct Stu {
        name: String,
    }
    #[test]
    fn test_serialize() {
        let j = Json(Stu {
            name: "hello".to_string(),
        });
        if let Ok(body) = ResponseBody::try_from(&j) {
            println!("{:?}", body);
        }
    }
    #[test]
    fn test_deserialize() {
        let mut req = HttpRequest::fake();
        let body = serde_json::to_vec(&Stu {
            name: "hello".to_string(),
        })
        .unwrap();
        *req._inner.body_mut() = Some(RequestBody::Simple(body.into()));
        if let Ok(j) = Json::<Stu>::try_convert_from(&mut req) {
            println!("{:?}", j);
        }
    }

    #[tokio::test]
    async fn constuct_response() {
        let mut res = HttpResponse::new();

        let mut header = HeaderMap::new();
        header.insert("wo", "cao".parse().unwrap());
        let res_line = StatusCode::OK;
        let j = Json(Stu {
            name: "hello".to_string(),
        });
        let mut a: Vec<Box<dyn HttpResponseModifier + Send + Sync>> = res_modifiers!(header, j);
        let _ = a.modify(&mut res).await;
        let mut a = Box::new(res_line);
        let _ = a.modify(&mut res).await;

        // header.modify(&mut res);
        // res_line.modify(&mut res);
        // j.modify(&mut res);
        println!("{:?}", res);
    }
}
