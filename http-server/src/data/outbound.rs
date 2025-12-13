use std::path::Path;

use bytes::Bytes;
use serde::Serialize;

use crate::{
    data::Json,
    map_str,
    response::{HttpResponseModifier, ResponseBody},
};

impl<T: Serialize + Send + Sync> HttpResponseModifier for Json<T> {
    fn modify<'a>(
        &'a self,
        res: &'a mut crate::response::HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), String>> + 'a + Send + Sync>> {
        Box::pin(async move {
            use ResponseBody::*;
            res.add_header(("content-type", "application/json"));
            let body = self.try_into()?;
            if let Simple(ref b) = body {
                res.add_header(("content-length", b.len().to_string().as_str()));
            }
            res.body = body;
            Ok(())
        })
    }
}
impl HttpResponseModifier for &str {
    fn modify<'a>(
        &'a self,
        res: &'a mut crate::response::HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), String>> + 'a + Send + Sync>> {
        Box::pin(async move {
            res.add_header(("content-type", "text/plain"));
            res.add_header(("content-length", self.as_bytes().len().to_string().as_str()));
            let b: Bytes = Bytes::from_iter(self.as_bytes().iter().copied());
            res.set_body(ResponseBody::Simple(b));
            Ok(())
        })
    }
}
impl HttpResponseModifier for String {
    fn modify<'a>(
        &'a self,
        res: &'a mut crate::response::HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), String>> + 'a + Send + Sync>> {
        Box::pin(async move {
            res.add_header(("content-type", "text/plain"));
            res.add_header(("content-length", self.as_bytes().len().to_string().as_str()));
            let b: Bytes = Bytes::from_iter(self.as_bytes().iter().copied());
            res.set_body(ResponseBody::Simple(b));
            Ok(())
        })
    }
}

pub struct StaticFile<T:AsRef<Path>>(pub T);

impl <T:AsRef<Path> + Send + Sync> HttpResponseModifier for StaticFile<T> {
    fn modify<'a>(
        &'a self,
        res: &'a mut crate::response::HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), String>> + 'a + Send + Sync>> {
        Box::pin(async move {
            let f = tokio::fs::File::open(self.0.as_ref()).await.map_err(map_str!())?;
            let meta = f.metadata().await.map_err(map_str!())?;
            if !meta.is_file() {
                return Err(format!("{:?} is not a File!!",self.0.as_ref()));
            }
            let len = meta.len();
            res.add_header(("content-length", len.to_string().as_str()));
            res.add_header(("content-type", "text/plain"));
            res.set_body(ResponseBody::File(f));
            Ok(())
        })
    }
}

impl<T: Serialize> TryFrom<&Json<T>> for ResponseBody {
    type Error = String;
    fn try_from(value: &Json<T>) -> Result<Self, Self::Error> {
        let res = serde_json::to_vec(value).map_err(map_str!())?;
        Ok(Self::Simple(Bytes::from(res)))
    }
}
