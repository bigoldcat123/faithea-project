

use std::{collections::HashMap, future::Future, pin::Pin};

use crate::{
    regulate_url_path, request::HttpRequest, response::HttpResponse, route::{Route, RouteComponent}
};

pub type Guard = Box<
    dyn Fn(
            HttpRequest,
        ) -> Pin<
            Box<dyn Future<Output = Result<HttpRequest, HttpResponse>> + Send + Sync + 'static>,
        > + Send
        + Sync
        + 'static,
>;


#[derive(Default)]
pub struct GuardTire {
    /// Child nodes in the trie, keyed by route components
    path: HashMap<RouteComponent, Box<Self>>,
    /// Guard function stored at this node (if this is a terminal node)
    f: Option<Guard>,
}

impl GuardTire {

    pub fn add<F, O, P>(&mut self, url: P, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpRequest, HttpResponse>> + 'static + Send + Sync,
        P: AsRef<str>
    {
        let url = regulate_url_path(url);
        let mut url_route = Route::from(url.as_str());
        url_route.r.reverse();
        self.add_url(url_route, f);
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
        let url = regulate_url_path(url);
        let chain = self.get_guard_chain(&url);
        let mut res = Some(req);
        for (_route, guard_fn) in chain {
            if let Some(req) = res.take() {
                match guard_fn(req).await {
                    Ok(req) => res = Some(req),
                    Err(res) => return Err(res),
                }
            }
        }
        Ok(res.unwrap())
    }


    fn get_guard_chain(&self, url: &str) -> Vec<(Route, &Guard)> {
        let url_parts: Vec<&str> = url.split("/").collect();
        let mut candidates: Vec<(Route, &Guard)> = vec![];
        self.get_candidates(&url_parts, &mut candidates, 0, Route { r: vec![] });
        candidates.sort_by(|a, b| a.0.cmp(&b.0).reverse());

        // Debug logging (commented out in production)
        // for route in candidates.iter().map(|x| &x.0) {
        //     println!("-> {:?}", route);
        // }

        candidates
    }

    fn get_candidates<'a>(
        &'a self,
        url_parts: &Vec<&str>,
        candidates: &mut Vec<(Route, &'a Guard)>,
        idx: usize,
        current_path: Route,
    ) {
        if idx < url_parts.len() {
            let url_part = url_parts[idx];
            for (component, child) in self.path.iter().filter(|(comp, _)| comp.match_url(url_part)) {
                // Debug logging (commented out in production)
                // println!("{}, {:?}", url_part, component);

                if *component == RouteComponent::MultiSegWildCard {
                    // Multi-segment wildcard matches the rest of the path
                    if let Some(f) = child.f.as_ref() {
                        let mut path = current_path.clone();
                        path.r.push(component.clone());
                        candidates.push((path, f));
                    }
                } else if idx + 1 < url_parts.len() {
                    // Continue matching deeper path segments
                    let mut path = current_path.clone();
                    path.r.push(component.clone());
                    child.get_candidates(url_parts, candidates, idx + 1, path);
                } else if let Some(f) = child.f.as_ref() {
                    // Reached the end of the URL, add guard if present
                    let mut path = current_path.clone();
                    path.r.push(component.clone());
                    candidates.push((path, f));
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::guard::GuardTire;


    #[test]
    fn test_guard_chain_ordering() {
        let mut guards = GuardTire::default();
        guards.add("/url/*/efg", async |e| Ok(e));
        guards.add("/url/abc/efg", async |e| Ok(e));
        guards.add("/url/**", async |e| Ok(e));
        guards.add("/url/abc", async |e| Ok(e));

        let chain = guards.get_guard_chain("/url/abc/efg");
        let routes: Vec<_> = chain.iter().map(|x| &x.0).collect();
        assert_eq!(r#"[Route { r: [Exact(""), Exact("url"), Exact("abc"), Exact("efg")] }, Route { r: [Exact(""), Exact("url"), SingleSegWildCard, Exact("efg")] }, Route { r: [Exact(""), Exact("url"), MultiSegWildCard] }]"#,
            format!("{:?}",routes));

    }
}
