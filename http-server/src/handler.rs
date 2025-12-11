use std::{collections::HashMap, pin::Pin};

use crate::{
    regulate_url_path, request::HttpRequest, response::HttpResponse, route::{Route, RouteComponent}
};
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
    path: HashMap<RouteComponent, Box<Self>>,
    f: Option<Fu>,
}
impl HandlerTire {
    pub fn add<F, O, P>(&mut self, url: P, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = HttpResponse> + 'static + Send + Sync,
        P: AsRef<str>
    {
        let url = regulate_url_path(url);
        let mut route = Route::try_from(url.as_str()).unwrap();
        route.r.reverse();
        self.add_url(route.r, f);
    }
    fn add_url<F, O>(&mut self, mut url: Vec<RouteComponent>, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = HttpResponse> + 'static + Send + Sync,
    {
        if let Some(next) = url.pop() {
            if !self.path.contains_key(&next) {
                self.path.insert(next.clone(), Default::default());
            }
            if url.is_empty() {
                self.path.get_mut(&next).unwrap().f =
                    Some(Box::new(move |r: HttpRequest| Box::pin(f(r))));
            } else {
                self.path.get_mut(&next).unwrap().add_url(url, f);
            }
        }
    }
    pub fn get(&self, url: &str) -> Option<(Route, &Fu)> {
        let url = regulate_url_path(url);
        let url: Vec<&str> = url.split("/").collect();
        let mut candidates: Vec<(Route, &Fu)> = vec![];
        self.get_candidates(&url, &mut candidates, 0, Route { r: vec![] });

        candidates.sort_by(|a, b| a.0.cmp(&b.0));
        // for i in candidates.iter().map(|x| &x.0).collect::<Vec<&Route>>() {
        //     println!("{:?}", i);
        // }

        candidates.pop()
    }
    fn get_candidates<'a>(
        &'a self,
        url: &Vec<&str>,
        candidates: &mut Vec<(Route, &'a Fu)>,
        idx: usize,
        current_path: Route,
    ) {
        if idx < url.len() {
            let url_part = url[idx];
            for n in self.path.iter().filter(|x| x.0.match_url(url_part)) {
                if idx + 1 < url.len() {
                    if *n.0 == RouteComponent::MutiSegWildCard {
                        if let Some(f) = n.1.f.as_ref() {
                            let mut p = current_path.clone();
                            p.r.push(n.0.clone());
                            candidates.push((p, f));
                        }
                    } else {
                        let mut p = current_path.clone();
                        p.r.push(n.0.clone());
                        n.1.get_candidates(url, candidates, idx + 1, p);
                    }
                } else if let Some(f) = n.1.f.as_ref() {
                    let mut p = current_path.clone();
                    p.r.push(n.0.clone());
                    candidates.push((p, f));
                }
            }
        }
    }
}
#[cfg(test)]
mod test {
    use crate::{handler::HandlerTire, request::HttpRequest, response::HttpResponse};

    async fn f(_: HttpRequest) -> HttpResponse {
        HttpResponse::new()
    }

    #[test]
    fn t1() {
        let mut handler = HandlerTire::default();
        handler.add("/url/abc/efg", f);
        handler.add("/url/{abc}/{efg}", f);
        handler.add("/url/abc", f);
        handler.add("/url/*/efg", f);
        handler.add("/url/**", f);
        let a = handler.get("/url/ab2c/efg").unwrap();
        assert_eq!("Route { r: [Exact(\"\"), Exact(\"url\"), PathParam(\"abc\"), PathParam(\"efg\")] }",format!("{:?}",a.0));
    }
}
