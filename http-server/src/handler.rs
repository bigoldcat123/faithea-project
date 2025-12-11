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
/// Route types (Beautified):
/// 1. Exact route
///    - Example: /hello/world
///    - Matches every path segment exactly. Highest priority.
/// 2. Path parameters
///    - Example: /hello/{name}
///    - Uses braces {} to capture a single path segment as a parameter (e.g. name).
/// 3. Single-segment wildcard
///    - Example: /hello/*/world
///    - A single asterisk * matches exactly one path segment (does not cross slashes).
/// 4. Multi-segment wildcard
///    - Example: /hello/**
///    - A double asterisk ** matches any number of subsequent path segments (including zero).
///
///
///Note: Typical matching precedence is —
///
/// Exact > Path parameters > Single-segment wildcard > Multi-segment wildcard.
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
    pub fn get(&self, url: &str) -> Option<(String, &Fu)> {
        let url: Vec<&str> = url.split("/").collect();
        let mut candidates: Vec<(String, &Fu)> = vec![];
        self.get_candidates(&url, &mut candidates, 0, String::new());
        for (s,_) in candidates.iter_mut() {
            *s = s.replace("{", "|");
        }
        candidates.sort_by(|a, b| {
            a.0.cmp(&b.0).reverse()
        });

        for (s,_) in candidates.iter_mut() {
            *s = s.replace("|", "{");
        }
        println!(
            "{:?}",
            candidates
                .iter()
                .map(|x| x.0.as_str())
                .collect::<Vec<&str>>()
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
            for n in self.path.iter().filter(|x| {
                x.0.ends_with("}") && x.0.starts_with("{")
                    || x.0 == url_part
                    || x.0 == "*"
                    || x.0 == "**"
            }) {
                if idx + 1 < url.len() {
                    if n.0 == "**" {
                        if let Some(f) = n.1.f.as_ref() {
                            candidates.push((format!("{}/{}", current_path, n.0), f));
                        }
                    }else {
                        n.1.get_candidates(
                            url,
                            candidates,
                            idx + 1,
                            format!("{}/{}", current_path, n.0),
                        );
                    }

                } else if let Some(f) = n.1.f.as_ref() {
                    candidates.push((format!("{}/{}", current_path, n.0), f));
                }
            }
        }
    }
}
#[cfg(test)]
mod test {
    use crate::{handler::HandlerTire, request::HttpRequest, response::HttpResponse};

    async fn f(r: HttpRequest) -> HttpResponse {
        HttpResponse::new()
    }

    #[test]
    fn t1() {
        let mut handler = HandlerTire::default();
        handler.add("/url/abc/efg".to_string(), f);
        handler.add("/url/{abc}/{efg}".to_string(),f);
        handler.add("/url/abc".to_string(), f);
        handler.add("/url/*/efg".to_string(), f);
         handler.add("/url/**".to_string(), f);
        let a = handler.get("/url/abc/efg").unwrap();
    }
}
