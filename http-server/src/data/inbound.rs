use bytes::Buf;
use serde::Deserialize;

use crate::{data::Json, map_str, request::HttpRequest};


impl <'a,T:Deserialize<'a>> TryFrom<&'a  HttpRequest> for Json<T> {
    type Error = String;
    fn try_from(value: &'a HttpRequest) -> Result<Self, Self::Error> {
        if let Some(body) = value.body.as_ref() {
            Ok(Self(serde_json::from_slice::<T>(body.chunk()).map_err(map_str!())?))
        }else {
            Err("()".to_string())
        }
    }
}


// impl TryFrom<&HttpRequest> for String {
//     type Error = String;
//     fn try_from(value: &HttpRequest) -> Result<Self, Self::Error> {

//     }
// }
