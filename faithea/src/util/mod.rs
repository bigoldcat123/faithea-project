use crate::{data::outbound::StaticFile, request::HttpRequest, res_modifiers, response::HttpResponseModifier};

/// # how to use
/// ``` rust
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
    if let Some(multi_seg_param) = _req.multi_seg_param.as_ref() {
        let a: StaticFile<String> = StaticFile(format!("{}/{}", path.as_ref(), multi_seg_param));
        res_modifiers!(a)
    } else {
        res_modifiers!("no multi_seg_param found try to use /abc/** route!")
    }
}
