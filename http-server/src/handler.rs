use std::{collections::HashMap, pin::Pin};

use crate::{request::HttpRequest, response::HttpResponse};
type Fu = Box<dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send + Sync + 'static>> + Send + Sync + 'static>;

#[derive(Default)]
pub struct Handler {
    handles: HashMap<String, Fu>,
}
impl Handler {
    pub fn add<F, O>(&mut self, url: String, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = HttpResponse> + 'static + Send + Sync,
    {
        let a: Fu = Box::new(move |r: HttpRequest| {
            Box::pin(f(r))
        });
        self.handles.insert(url, a);
    }
    pub fn get(&self,url:&str) -> Option<&Fu> {
        self.handles.get(url)
    }
    pub fn urls(&self)  {

    }
}
