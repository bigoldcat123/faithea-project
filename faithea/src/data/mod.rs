use bytes::Buf;
use serde::{Deserialize, Serialize};

use crate::{
    handler::HttpHandlerError,
    request::{HttpRequest, RequestBody},
};

pub mod inbound;
pub mod outbound;

#[derive(Serialize, Debug)]
pub struct Json<T>(pub T);

impl<'a, T: Deserialize<'a>> TryFrom<&'a HttpRequest> for Json<T> {
    type Error = HttpHandlerError;
    fn try_from(value: &'a HttpRequest) -> Result<Self, Self::Error> {
        if let Some(RequestBody::Simple(body)) = value._inner.body() {
            Ok(Self(serde_json::from_slice::<T>(body.chunk()).map_err(
                |x| {
                    let a: Self::Error = Box::new(x.to_string());
                    a
                },
            )?))
        } else {
            let err = Box::new("Json parsing error!");
            Err(err)
        }
    }
}
impl<'a, T: Deserialize<'a>> TryFrom<&'a mut HttpRequest> for Json<T> {
    type Error = HttpHandlerError;
    fn try_from(value: &'a mut HttpRequest) -> Result<Self, Self::Error> {
        let im_ref = &(*value);
        im_ref.try_into()
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
        request::HttpRequest,
        res_modifiers,
        response::{HttpResponse, HttpResponseModifier, ResponseBody},
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
        if let Ok(j) = Json::<Stu>::try_from(&req) {
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
