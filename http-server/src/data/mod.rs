

use bytes::Buf;
use serde::{Deserialize, Serialize};

use crate::{map_str, request::HttpRequest};

pub mod inbound;
pub mod outbound;

#[derive(Serialize, Debug)]
pub struct Json<T>(pub T);

impl <'a,T:Deserialize<'a>> TryFrom<&'a  HttpRequest> for Json<T> {
    type Error = String;
    fn try_from(value: &'a HttpRequest) -> Result<Self, Self::Error> {
        if let Some(body) = value.body.as_ref() {
            Ok(Self(serde_json::from_slice::<T>(body.chunk()).map_err(map_str!())?))
        }else {
            Err("Json parsing error!".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::{
        HttpHeader,
        request::HttpRequest,
        res_modifiers,
        response::{HttpResponse, HttpResponseModifier, ResponseBody, ResponseStatusLine},
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
        let body = ResponseBody::try_from(&j).unwrap();
        println!("{:?}", body);
    }
    #[test]
    fn test_deserialize() {
        fn hello(_: Json<Stu>) {}
        let mut req = HttpRequest::fake();
        let body = serde_json::to_vec(&Stu {
            name: "hello".to_string(),
        })
        .unwrap();
        req.body = Some(body.into());
        let j: Json<Stu> = Json::try_from(&req).unwrap();
        println!("{:?}", j);
        hello(Json::try_from(&req).unwrap());
    }

    #[tokio::test]
    async fn constuct_response() {
        let mut res = HttpResponse::new();

        let mut header = HttpHeader::new();
        header.add("wo".into(), "cao".into());
        let res_line = ResponseStatusLine::new("a", "b", "c");
        let j = Json(Stu {
            name: "hello".to_string(),
        });
        let mut a: Vec<Box<dyn HttpResponseModifier + Send + Sync>> = res_modifiers!(header, j);
        a.modify(&mut res).await.unwrap();
        let mut a = Box::new(res_line);
        a.modify(&mut res).await.unwrap();

        // header.modify(&mut res);
        // res_line.modify(&mut res);
        // j.modify(&mut res);
        println!("{:?}", res);
    }
}
