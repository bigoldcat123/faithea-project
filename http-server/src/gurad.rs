use std::{collections::HashMap, pin::Pin};

use crate::{request::HttpRequest, response::HttpResponse};

type Guard = Box<
    dyn Fn(
            HttpRequest,
        ) -> Pin<
            Box<dyn Future<Output = Result<HttpRequest, HttpResponse>> + Send + Sync + 'static>,
        > + Send
        + Sync
        + 'static,
>;
#[derive(Default)]
pub struct GurardTire {
    path: HashMap<String, Box<Self>>,
    f: Option<Guard>,
}
impl GurardTire {
    pub fn add<F, O>(&mut self, url: String, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpRequest, HttpResponse>> + 'static + Send + Sync,
    {
        let mut url: Vec<&str> = url.split("/").collect();
        url.reverse();
        self.add_url(url, f);
    }
    fn add_url<F, O>(&mut self, mut url: Vec<&str>, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpRequest, HttpResponse>> + 'static + Send + Sync,
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
    pub async fn guard(&self, url: &str, req: HttpRequest) -> Result<HttpRequest, HttpResponse> {
        let chain = self.get(url);
        let mut res = Some(req);
        for c in chain {
            if let Some(req) = res.take() {
                match c.1(req).await {
                    Ok(req) => res = Some(req),
                    Err(res) => return Err(res),
                }
            }
        }
        Ok(res.unwrap())
    }
    fn get(&self, url: &str) -> Vec<(String, &Guard)> {
        let url: Vec<&str> = url.split("/").collect();
        let mut candidates: Vec<(String, &Guard)> = vec![];
        self.get_candidates(&url, &mut candidates, 0, String::new());

        println!(
            "{:?}",
            candidates
                .iter()
                .map(|x| x.0.as_str())
                .collect::<Vec<&str>>()
        );
        candidates
    }
    fn get_candidates<'a>(
        &'a self,
        url: &Vec<&str>,
        candidates: &mut Vec<(String, &'a Guard)>,
        idx: usize,
        current_path: String,
    ) {
        if idx < url.len() {
            let url_part = url[idx];
            for n in self
                .path
                .iter()
                .filter(|x| x.0 == "*" || x.0 == "**" || x.0 == url_part)
            {
                if n.0 == "**" {
                    if let Some(f) = n.1.f.as_ref() {
                        candidates.push((format!("{}/{}", current_path, n.0), f));
                    }
                } else if idx + 1 < url.len() {
                    n.1.get_candidates(
                        url,
                        candidates,
                        idx + 1,
                        format!("{}/{}", current_path, n.0),
                    );
                } else if let Some(f) = n.1.f.as_ref() {
                    candidates.push((format!("{}/{}", current_path, n.0), f));
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::gurad::GurardTire;

    #[test]
    fn t1() {
        let mut g = GurardTire::default();
        g.add("/url/*/efg".to_string(), async |e| Ok(e));
        g.add("/url/abc/efg".to_string(), async |e| Ok(e));
        g.add("/url/**".to_string(), async |e| Ok(e));
        g.add("/url/abc".to_string(), async |e| Ok(e));
        g.get("/url/abc/efg/asdas");
    }
}
