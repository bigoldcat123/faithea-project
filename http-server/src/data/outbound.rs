use std::{ffi::OsStr, path::Path};

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
            res.add_header(("content-type", mime_type(self.0.as_ref())));
            res.set_body(ResponseBody::File(f));
            Ok(())
        })
    }
}
fn mime_type(path:&Path) -> &str {
    match path.extension() {
        Some(e) => get_mime_type_by_extention(e),
        None    => "application/octet-stream"
    }
}
fn get_mime_type_by_extention(e:&OsStr) -> &str {
    if let Some(e) = e.to_str() {
        match e {
            "html" => "text/html",
            "htm"  => "text/html",
            "css"  => "text/css",
            "js"   => "text/javascript",
            "json" => "application/json",
            "map"  => "application/json",
            // image
            "png"  => "image/png",
            "jpg"  => "image/jpeg",
            "jpeg" => "image/jpeg",
            "gif"  => "image/gif",
            "webp" => "image/webp",
            "svg"  => "image/svg+xml",
            "ico"  => "image/x-icon",
            //file
            "txt"  => "text/plain",
            "md"   => "text/markdown",
            "csv"  => "text/csv",
            "xml"  => "application/xml",
            "pdf"  => "application/pdf",
            // video
            "mp3"  => "audio/mpeg",
            "wav"  => "audio/wav",
            "ogg"  => "audio/ogg",
            "mp4"  => "video/mp4",
            "webm" => "video/webm",
            // zips
            "zip"  => "application/zip",
            "tar"  => "application/x-tar",
            "gz"   => "application/gzip",
            "7z"   => "application/x-7z-compressed",
            // font
            "woff"  => "font/woff",
            "woff2" => "font/woff2",
            "ttf"   => "font/ttf",
            "otf"   => "font/otf",
            _   => "application/octet-stream"
        }
    }else {
        "application/octet-stream"
    }
}

impl<T: Serialize> TryFrom<&Json<T>> for ResponseBody {
    type Error = String;
    fn try_from(value: &Json<T>) -> Result<Self, Self::Error> {
        let res = serde_json::to_vec(value).map_err(map_str!())?;
        Ok(Self::Simple(Bytes::from(res)))
    }
}
