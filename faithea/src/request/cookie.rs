use std::collections::HashMap;


#[derive(Default, Debug)]
pub struct Cookie<'a> {
    _inner: HashMap<&'a str, &'a str>,
}
impl<'a> Cookie<'a> {
    pub(crate) fn from_cookie_header(s: &'a str) -> Self {
        let mut map = HashMap::new();
        for (k, v) in s
            .split(";")
            .filter_map(|x| x.split_once("="))
            .map(|(k, v)| (k.trim(), v.trim()))
        {
            map.insert(k, v);
        }
        Self { _inner: map }
    }
}
