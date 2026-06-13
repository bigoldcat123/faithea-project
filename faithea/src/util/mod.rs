pub(crate) mod trie;
use std::path::Path;

use crate::{
    data::outbound::StaticFile, error::Error, request::HttpRequest, res_modifiers, response::HttpResponseModifier
};

/// # how to use
/// ``` ignore
/// use faithea::{get, util::static_map};
///
/// #[get("/**")]
/// pub async fn file_map() {
///     static_map(_req,"path/to/static/directory").await;
/// }
/// ```
pub async fn static_map<P: AsRef<str>>(
    _req: &HttpRequest,
    path: P,
) -> Vec<Box<dyn HttpResponseModifier + Send + Sync>> {
    if let Some(multi_seg_param) = _req.multi_seg_param.as_ref()
        && let Ok(multi_seg_param) = urlencoding::decode(multi_seg_param)
    {
        // /hello/something
        // 1. /hello/something
        // 2. /hello/something/index.html
        // 3. /hello/something.html
        //
        match Exact::parse(path, multi_seg_param) {
            Ok(res) => res,
            Err(e) => {
                res_modifiers!(e)
            }
        }
    } else {
        res_modifiers!("no multi_seg_param found try to use /abc/** route!")
    }
}
struct Exact;
impl Exact {
    fn parse<P: AsRef<str>, S: AsRef<str>>(
        p: P,
        seg: S,
    ) -> Result<Vec<Box<dyn HttpResponseModifier + Send + Sync>>, Error> {
        let path_str = format!("{}/{}", p.as_ref(), seg.as_ref());
        let path = Path::new(&path_str);
        if path.exists() && path.is_file() {
            return Ok(res_modifiers!(StaticFile(path_str)));
        } else {
            Index::parse(p, seg)
        }
    }
}
struct Index;
impl Index {
    fn parse<P: AsRef<str>, S: AsRef<str>>(
        p: P,
        seg: S,
    ) -> Result<Vec<Box<dyn HttpResponseModifier + Send + Sync>>, Error> {
        let mut path_str = format!("{}/{}", p.as_ref(), seg.as_ref());
        if path_str.ends_with("/") {
            path_str.push_str("index.html");
        }else {
            path_str.push_str("/index.html");
        }

        let path = Path::new(&path_str);
        if path.exists() && path.is_file() {
            return Ok(res_modifiers!(StaticFile(path_str)));
        } else {
            HTML::parse(p, seg)
        }
    }
}

struct HTML;
impl HTML {
    fn parse<P: AsRef<str>, S: AsRef<str>>(
        path: P,
        seg: S,
    ) -> Result<Vec<Box<dyn HttpResponseModifier + Send + Sync>>, Error> {
        let mut path_str = format!("{}/{}", path.as_ref(), seg.as_ref());
        if path_str.ends_with("/") {
            path_str.pop();
        }
        path_str.push_str(".html");

        let path = Path::new(&path_str);
        if path.exists() && path.is_file() {
            return Ok(res_modifiers!(StaticFile(path_str)));
        } else {
            Err(Error::before_handler_invalid_param("static file not exit"))
        }
    }
}
