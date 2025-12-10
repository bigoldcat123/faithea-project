use std::{collections::HashMap, pin::Pin};

use crate::{request::HttpRequest, response::HttpResponse};
type Fu = Box<
    dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send + Sync + 'static>>
        + Send
        + Sync
        + 'static,
>;

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
        let a: Fu = Box::new(move |r: HttpRequest| Box::pin(f(r)));
        self.handles.insert(url, a);
    }
    pub fn get(&self, url: &str) -> Option<&Fu> {
        self.handles.get(url)
    }
}

#[derive(Default)]
pub struct HandlerTire {
    path: HashMap<String, Box<Self>>,
    f: Option<Fu>,
}
impl HandlerTire {
    pub fn add<F, O>(&mut self, url: String, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = HttpResponse> + 'static + Send + Sync,
    {

        let mut url: Vec<&str> = url.split("/").collect();
        url.reverse();
        self.add_url(url, f);
    }
    fn add_url<F, O>(&mut self, mut url: Vec<&str>, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = HttpResponse> + 'static + Send + Sync,
    {
        if let Some(next) = url.pop() {
            if !self.path.contains_key(next) {
                self.path.insert(next.to_string(), Default::default());
            }
            if url.is_empty() {
                self.path.get_mut(next).unwrap().f =
                    Some(Box::new(move |r: HttpRequest| Box::pin(f(r))));
            } else {
                self.path.get_mut(next).unwrap().add_url(url, f);
            }
        }
    }
    pub fn get(&self, url: &str) -> Option<(String,&Fu)> {
        let url: Vec<&str> = url.split("/").collect();
        let mut candidates: Vec<(String, &Fu)> = vec![];
        self.get_candidates(&url, &mut candidates, 0, String::new());

        candidates.sort_by(|a,b| {
            if a.0.starts_with("{") && b.0.starts_with("{") {
                std::cmp::Ordering::Equal
            }else if a.0.starts_with("{")  {
                std::cmp::Ordering::Greater
            }else if b.0.starts_with("{")  {
                std::cmp::Ordering::Less
            }else {
                std::cmp::Ordering::Equal
            }
        });
        println!(
            "{:?}",
            candidates.iter().map(|x| x.0.as_str()).collect::<Vec<&str>>()
        );
        candidates.pop()
    }
    fn get_candidates<'a>(
        &'a self,
        url: &Vec<&str>,
        candidates: &mut Vec<(String, &'a Fu)>,
        idx: usize,
        current_path: String,
    ) {
        if idx < url.len() {
            let url_part = url[idx];
            for n in self
                .path
                .iter()
                .filter(|x| x.0.ends_with("}") && x.0.starts_with("{") || x.0 == url_part)
            {
                if idx + 1 < url.len() {
                    n.1.get_candidates(
                        url,
                        candidates,
                        idx + 1,
                        format!("{}/{}", current_path, n.0),
                    );
                } else {
                    if let Some(f) = n.1.f.as_ref() {
                        candidates.push((format!("{}/{}", current_path, n.0), f));
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod test {
    use crate::{handler::HandlerTire, response::HttpResponse};

    #[test]
    fn t1() {
        let mut handler = HandlerTire::default();
        handler.add("/url/abc/efg".to_string(), async |_| HttpResponse::new());
        handler.add("/url/{abc}/{efg}".to_string(), async |_| HttpResponse::new());
        handler.add("/url/abc".to_string(), async |_| HttpResponse::new());
        let a = handler.get("/url/abc/efg").unwrap();
        assert_eq!(a.0,"//url/abc/efg");
        let a = handler.get("/url/abc/asd").unwrap();

        assert_eq!(a.0,"//url/{abc}/{efg}")
    }
}
