use std::path::Path;

use bytes::Bytes;
use serde::Serialize;

use crate::{
    data::Json,
    map_str,
    response::{HttpResponseModifier, ResponseBody},
};

impl<T: Serialize> HttpResponseModifier for Json<T> {
    fn modify(&self, res: &mut crate::response::HttpResponse) -> Result<(), String> {
        use ResponseBody::*;
        res.add_header(("content-type", "application/json"));
        let body = self.try_into()?;
        if let Simple(ref b) = body {
            res.add_header(("content-length", b.len().to_string().as_str()));
        }
        res.body = body;
        Ok(())
    }
}
impl HttpResponseModifier for &str {
    fn modify(&self, res: &mut crate::response::HttpResponse) -> Result<(), String> {
        res.add_header(("content-type", "application/text"));
        res.add_header(("content-length", self.as_bytes().len().to_string().as_str()));
        let b:Bytes = Bytes::from_iter(self.as_bytes().iter().copied());
        res.set_body(ResponseBody::Simple(b));
        Ok(())
    }
}

pub struct  StaticFile<'a>(pub  &'a Path);

// impl <'a> HttpResponseModifier for StaticFile<'a> {
//     fn modify(&self, res: &mut crate::response::HttpResponse) -> Result<(), String> {
//         File::open(self.0).await.map_err(map_str!())?;
//     }
// }


impl<T: Serialize> TryFrom<&Json<T>> for ResponseBody {
    type Error = String;
    fn try_from(value: &Json<T>) -> Result<Self, Self::Error> {
        let res = serde_json::to_vec(value).map_err(map_str!())?;
        Ok(Self::Simple(Bytes::from(res)))
    }
}
