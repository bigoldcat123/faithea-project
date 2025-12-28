use crate::{data::outbound::StaticFile, request::HttpRequest, res_modifiers, response::HttpResponseModifier};

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
