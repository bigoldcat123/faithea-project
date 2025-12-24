use std::{collections::HashMap, future::Future, pin::Pin};

use http::Method;

use crate::{
    regulate_url_path,
    request::{HttpRequest},
    response::{HttpResponse, HttpResponseModifier},
    route::{Route, RouteComponent}, server::HandlerModifier,
};
pub type FuError = Box<dyn HttpResponseModifier + Send + Sync>;
pub type Fu = Box<
    dyn Fn(
            HttpRequest,
        )
            -> Pin<Box<dyn Future<Output = Result<HttpResponse, FuError>> + Send + Sync + 'static>>
        + Send
        + Sync
        + 'static,
>;

#[derive(Default)]
pub struct HandlerTire {
    /// Child nodes in the routing trie, keyed by route components
    path: HashMap<RouteComponent, Box<Self>>,
    /// Handler function stored at this node (if this is a terminal node)
    f: HashMap<Method, Fu>,
}
impl HandlerTire {
    /// m just format!("{}{}",route,pre_fix)
    /// so here to make sure pre_fix is not '/'-ended!,since route is '/'-started
    ///
    pub fn mount(&mut self, pre_fix: &'static str, handlers: Vec<HandlerModifier>) {
        let pre_fix = if let Some(pre_fix) = pre_fix.strip_suffix("/") {
            pre_fix
        }else {
            pre_fix
        };
        for m in handlers {
            m(self, pre_fix);
        }
    }

    pub fn get<F, O, P>(&mut self, url: P, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpResponse, FuError>> + 'static + Send + Sync,
        P: AsRef<str>,
    {
        let url = regulate_url_path(url);
        let mut route = Route::from(url.as_str());
        route.r.reverse();
        self.add_route(
            route.r,
            Box::new(move |r: HttpRequest| Box::pin(f(r))),
            Method::GET,
        );
    }
    pub fn post<F, O, P>(&mut self, url: P, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpResponse, FuError>> + 'static + Send + Sync,
        P: AsRef<str>,
    {
        let url = regulate_url_path(url);
        let mut route = Route::from(url.as_str());
        route.r.reverse();
        self.add_route(
            route.r,
            Box::new(move |r: HttpRequest| Box::pin(f(r))),
            Method::POST,
        );
    }

    pub fn options<F, O, P>(&mut self, url: P, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpResponse, FuError>> + 'static + Send + Sync,
        P: AsRef<str>,
    {
        let url = regulate_url_path(url);
        let mut route = Route::from(url.as_str());
        route.r.reverse();
        self.add_route(
            route.r,
            Box::new(move |r: HttpRequest| Box::pin(f(r))),
            Method::OPTIONS,
        );
    }

    fn add_route(&mut self, mut url: Vec<RouteComponent>, f: Fu, method: Method) {
        if let Some(next) = url.pop() {
            if !self.path.contains_key(&next) {
                self.path.insert(next.clone(), Default::default());
            }
            if url.is_empty() {
                self.path
                    .get_mut(&next)
                    .unwrap()
                    .f
                    .insert(method, f);
            } else {
                self.path.get_mut(&next).unwrap().add_route(url, f, method);
            }
        }
    }

    pub fn get_handler(&self, url: &str, method: Method) -> Option<(Route, &Fu)> {
        let url = if let Some((url, _search)) = url.split_once("?") {
            url
        } else {
            url
        };
        let url = regulate_url_path(url);
        let url_parts: Vec<&str> = url.split("/").collect();
        let mut candidates: Vec<(Route, &Fu)> = vec![];
        self.get_candidates(
            &url_parts,
            &mut candidates,
            0,
            Route { r: vec![] },
            method,
        );

        candidates.sort_by(|a, b| a.0.cmp(&b.0));
        // Debug logging (commented out in production):
        // for route in candidates.iter().map(|x| &x.0) {
        //     println!("Candidate route: {:?}", route);
        // }

        candidates.pop()
    }

    fn get_candidates<'a>(
        &'a self,
        url_parts: &Vec<&str>,
        candidates: &mut Vec<(Route, &'a Fu)>,
        idx: usize,
        current_path: Route,
        method: Method,
    ) {
        if idx < url_parts.len() {
            let url_part = url_parts[idx];
            for (component, child) in self
                .path
                .iter()
                .filter(|(comp, _)| comp.match_url(url_part))
            {
                if idx + 1 < url_parts.len() {
                    if *component == RouteComponent::MultiSegWildCard {
                        // Multi-segment wildcard matches the rest of the path
                        if let Some(f) = child.f.get(&method) {
                            let mut path = current_path.clone();
                            path.r.push(component.clone());
                            candidates.push((path, f));
                        }
                    } else {
                        // Continue matching deeper path segments
                        let mut path = current_path.clone();
                        path.r.push(component.clone());
                        child.get_candidates(url_parts, candidates, idx + 1, path, method.clone());
                    }
                } else if let Some(f) = child.f.get(&method) {
                    // Reached the end of the URL, add handler if present
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
    use http::Method;

    use crate::{handler::{FuError, HandlerTire}, request::HttpRequest, response::HttpResponse};

    /// Test handler that returns a default response.
    async fn test_handler(_: HttpRequest) -> Result<HttpResponse, FuError> {
        Ok(HttpResponse::new())
    }

    /// Tests that route matching follows the correct precedence rules.
    ///
    /// This test verifies that when multiple patterns match a URL, the
    /// most specific one (according to Exact > PathParam > SingleSegWildCard > MultiSegWildCard)
    /// is selected.
    #[test]
    fn test_route_precedence() {
        let mut handler = HandlerTire::default();

        // Register various route patterns
        handler.get("/url/abc/efg", test_handler);
        handler.get("/url/{abc}/{efg}", test_handler);
        handler.get("/url/abc", test_handler);
        handler.get("/url/*/efg", test_handler);
        handler.get("/url/**", test_handler);

        // Test URL that matches multiple patterns
        let (matched_route, _) = handler.get_handler("/url/ab2c/efg", Method::GET).unwrap();

        // Should match the path parameter pattern, not the wildcards
        assert_eq!(
            "Route { r: [Exact(\"\"), Exact(\"url\"), PathParam(\"abc\"), PathParam(\"efg\")] }",
            format!("{:?}", matched_route)
        );
    }
}
