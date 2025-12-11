use std::{collections::HashMap, pin::Pin};

use crate::{
    regulate_url_path, request::HttpRequest, response::HttpResponse, route::{Route, RouteComponent}
};

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
    path: HashMap<RouteComponent, Box<Self>>,
    f: Option<Guard>,
}
impl  GurardTire {
    pub fn add<F, O, P>(&mut self, url: P, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpRequest, HttpResponse>> + 'static + Send + Sync,
        P:AsRef<str>
    {
        let url = regulate_url_path(url);
        let mut url = Route::try_from(url.as_str()).unwrap();
        url.r.reverse();
        self.add_url(url, f);
    }
    fn add_url<F, O>(&mut self, mut url: Route, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpRequest, HttpResponse>> + 'static + Send + Sync,
    {
        if let Some(next) = url.r.pop() {
            if !self.path.contains_key(&next) {
                self.path.insert(next.clone(), Default::default());
            }
            if url.r.is_empty() {
                self.path.get_mut(&next).unwrap().f =
                    Some(Box::new(move |r: HttpRequest| Box::pin(f(r))));
            } else {
                self.path.get_mut(&next).unwrap().add_url(url, f);
            }
        }
    }
    pub async fn guard(&self, url: &str, req: HttpRequest) -> Result<HttpRequest, HttpResponse> {
        let  url = regulate_url_path(url);
        let chain = self.get_gurad_chain(&url);
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
    fn get_gurad_chain(&self, url: &str) -> Vec<(Route, &Guard)> {
        let url: Vec<&str> = url.split("/").collect();
        let mut candidates: Vec<(Route, &Guard)> = vec![];
        self.get_candidates(&url, &mut candidates, 0, Route { r: vec![] });
        candidates.sort_by(|a,b|a.0.cmp(&b.0).reverse());
        for i in candidates.iter().map(|x| &x.0).collect::<Vec<&Route>>() {
            println!("-> {:?}", i);
        }

        candidates
    }
    fn get_candidates<'a>(
        &'a self,
        url: &Vec<&str>,
        candidates: &mut Vec<(Route, &'a Guard)>,
        idx: usize,
        current_path: Route,
    ) {
        if idx < url.len() {
            let url_part = url[idx];
            for n in self.path.iter().filter(|x| x.0.match_url(url_part)) {
                println!("{url_part}, {:?}",n.0);
                if *n.0 == RouteComponent::MutiSegWildCard {
                    if let Some(f) = n.1.f.as_ref() {
                        let mut path = current_path.clone();
                        path.r.push(n.0.clone());
                        candidates.push((path, f));
                    }
                } else if idx + 1 < url.len() {
                    let mut path = current_path.clone();
                    path.r.push(n.0.clone());
                    n.1.get_candidates(url, candidates, idx + 1, path);
                } else if let Some(f) = n.1.f.as_ref() {
                    let mut path = current_path.clone();
                    path.r.push(n.0.clone());
                    candidates.push((path, f));
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
        g.add("/url/*/efg", async |e| Ok(e));
        g.add("/url/abc/efg", async |e| Ok(e));
        g.add("/url/**", async |e| Ok(e));
        g.add("/url/abc", async |e| Ok(e));
        let a = g.get_gurad_chain("/url/abc/efg");
        assert_eq!(r#"[Route { r: [Exact(""), Exact("url"), Exact("abc"), Exact("efg")] }, Route { r: [Exact(""), Exact("url"), SingleSegWilCard, Exact("efg")] }, Route { r: [Exact(""), Exact("url"), MutiSegWildCard] }]"#,
            format!("{:?}",a.iter().map(|x|&x.0).collect::<Vec<_>>()));
    }
}
