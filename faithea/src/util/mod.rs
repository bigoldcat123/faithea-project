use std::path::Path;

use crate::{
    data::outbound::StaticFile, request::HttpRequest, res_modifiers, response::{HttpResponseModifier, redirect::Redirect},
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
        let file_path = format!("{}/{}", path.as_ref(), multi_seg_param);
        if Path::new(&file_path).is_dir() {
            let redirect_path = format!("{}/index.html", _req.uri().path());
            let r = Redirect(redirect_path);
            res_modifiers!(r)
        } else {
            let a: StaticFile<String> = StaticFile(file_path);
            res_modifiers!(a)
        }
    } else {
        res_modifiers!("no multi_seg_param found try to use /abc/** route!")
    }
}
