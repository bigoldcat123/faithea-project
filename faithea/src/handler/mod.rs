use std::collections::HashMap;

use http::Method;

use crate::{
    handler::types::{
        Handler, HttpHandlerFuture, RawHttpHandlerTrait, RawWebSocketHandlerTrait,
        WebSocketHandlerFuture,
    },
    normalize_url_path,
    request::HttpRequest,
    response::HttpResponse,
    route::{Route, RouteComponent},
    server::HandlerModifier,
    util::trie::Trie,
};
pub mod types;

pub type HandlerTrie = Trie<HashMap<Method, Handler>, RouteComponent>;
// #[derive(Default)]
// pub struct HandlerTrie {
//     /// Child nodes in the routing trie, keyed by route components
//     path: HashMap<RouteComponent, Box<Self>>,
//     /// Handler function stored at this node (if this is a terminal node)
//     f: HashMap<Method, Handler>,
// }
impl HandlerTrie {
    /// Mount handlers under a prefix, removing its trailing slash before joining routes.
    pub fn mount(&mut self, prefix: &'static str, handlers: Vec<HandlerModifier>) {
        let prefix = if let Some(prefix) = prefix.strip_suffix("/") {
            prefix
        } else {
            prefix
        };
        for m in handlers {
            m(self, prefix);
        }
    }

    pub fn get<F, O, P>(&mut self, url: P, f: F)
    where
        F: RawHttpHandlerTrait<O>,
        O: HttpHandlerFuture,
        P: AsRef<str>,
    {
        let url = normalize_url_path(url);
        let mut route = Route::from(url.as_str());
        route.components.reverse();
        self.add_route(
            route.components,
            Handler::Http(Box::new(move |request: HttpRequest| Box::pin(f(request)))),
            Method::GET,
        );
    }
    pub fn post<F, O, P>(&mut self, url: P, f: F)
    where
        F: RawHttpHandlerTrait<O>,
        O: HttpHandlerFuture,
        P: AsRef<str>,
    {
        let url = normalize_url_path(url);
        let mut route = Route::from(url.as_str());
        route.components.reverse();
        self.add_route(
            route.components,
            Handler::Http(Box::new(move |request: HttpRequest| Box::pin(f(request)))),
            Method::POST,
        );
    }

    pub fn put<F, O, P>(&mut self, url: P, f: F)
    where
        F: RawHttpHandlerTrait<O>,
        O: HttpHandlerFuture,
        P: AsRef<str>,
    {
        let url = normalize_url_path(url);
        let mut route = Route::from(url.as_str());
        route.components.reverse();
        self.add_route(
            route.components,
            Handler::Http(Box::new(move |request: HttpRequest| Box::pin(f(request)))),
            Method::PUT,
        );
    }

    pub fn delete<F, O, P>(&mut self, url: P, f: F)
    where
        F: RawHttpHandlerTrait<O>,
        O: HttpHandlerFuture,
        P: AsRef<str>,
    {
        let url = normalize_url_path(url);
        let mut route = Route::from(url.as_str());
        route.components.reverse();
        self.add_route(
            route.components,
            Handler::Http(Box::new(move |request: HttpRequest| Box::pin(f(request)))),
            Method::DELETE,
        );
    }

    pub fn options<F, O, P>(&mut self, url: P, f: F)
    where
        F: RawHttpHandlerTrait<O>,
        O: HttpHandlerFuture,
        P: AsRef<str>,
    {
        let url = normalize_url_path(url);
        let mut route = Route::from(url.as_str());
        route.components.reverse();
        self.add_route(
            route.components,
            Handler::Http(Box::new(move |request: HttpRequest| Box::pin(f(request)))),
            Method::OPTIONS,
        );
    }

    // pub(crate) fn proxy<P: AsRef<str>>(&mut self, url: P, target: &str) {
    //     let url = normalize_url_path(url);
    //     let proxy = Proxy::new(target);
    //     for method in [
    //         Method::GET,
    //         Method::POST,
    //         Method::PUT,
    //         Method::DELETE,
    //         Method::PATCH,
    //         Method::HEAD,
    //         Method::OPTIONS,
    //         Method::TRACE,
    //     ] {
    //         let mut route = Route::from(url.as_str());
    //         route.components.reverse();
    //         let proxy = proxy.clone();
    //         self.add_route(
    //             route.components,
    //             Handler::Http(Box::new(move |req| {
    //                 let proxy = proxy.clone();
    //                 Box::pin(async move { Ok(proxy.forward(req).await) })
    //             })),
    //             method,
    //         );
    //     }
    // }
    // Fn(
    //         Receiver<WebSocketPayload>,
    //         Sender<WebSocketPayload>,
    //         HttpRequest,
    //     ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
    pub fn websocket_h1<P: AsRef<str>, F, R>(&mut self, url: P, ws_handler: F)
    where
        F: RawWebSocketHandlerTrait<R>,
        R: WebSocketHandlerFuture,
    {
        let url = normalize_url_path(url);
        let mut route = Route::from(url.as_str());
        route.components.reverse();
        self.add_route(
            route.components,
            Handler::WebSocket(Box::new(move |ws, req| Box::pin(ws_handler(ws, req)))),
            Method::GET,
        );
    }

    pub fn websocket_h2<P: AsRef<str>, F, R>(&mut self, url: P, ws_handler: F)
    where
        F: RawWebSocketHandlerTrait<R>,
        R: WebSocketHandlerFuture,
    {
        let url = normalize_url_path(url);
        let mut route = Route::from(url.as_str());
        route.components.reverse();
        self.add_route(
            route.components,
            Handler::WebSocket(Box::new(move |ws, req| Box::pin(ws_handler(ws, req)))),
            Method::CONNECT,
        );
    }
    fn add_route(&mut self, mut url: Vec<RouteComponent>, f: Handler, method: Method) {
        if let Some(next) = url.pop() {
            if !self.children.contains_key(&next) {
                self.children.insert(next.clone(), Default::default());
            }
            assert!(self.children.get_mut(&next).is_some());
            if url.is_empty() {
                self.children
                    .get_mut(&next)
                    .unwrap()
                    .value
                    .insert(method, f);
                let handler: Handler =
                    Handler::Http(Box::new(|_| Box::pin(async { Ok(HttpResponse::new()) })));
                self.children
                    .get_mut(&next)
                    .unwrap()
                    .value
                    .insert(Method::HEAD, handler);
            } else {
                self.children
                    .get_mut(&next)
                    .unwrap()
                    .add_route(url, f, method);
            }
        }
    }

    pub fn get_handler(&self, url: &str, method: Method) -> Option<(Route, &Handler)> {
        let url = if let Some((url, _search)) = url.split_once("?") {
            url
        } else {
            url
        };
        let url = normalize_url_path(url);
        let url_parts: Vec<&str> = url.split("/").collect();
        let mut candidates: Vec<(Route, &Handler)> = vec![];
        self.get_candidates(
            &url_parts,
            &mut candidates,
            0,
            Route { components: vec![] },
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
        candidates: &mut Vec<(Route, &'a Handler)>,
        idx: usize,
        current_path: Route,
        method: Method,
    ) {
        if idx < url_parts.len() {
            let url_part = url_parts[idx];
            for (component, child) in self
                .children
                .iter()
                .filter(|(comp, _)| comp.match_url(url_part))
            {
                if idx + 1 < url_parts.len() {
                    if *component == RouteComponent::MultiSegmentWildcard {
                        // Multi-segment wildcard matches the rest of the path
                        if let Some(f) = child.value.get(&method) {
                            let mut path = current_path.clone();
                            path.components.push(component.clone());
                            candidates.push((path, f));
                        }
                    } else {
                        // Continue matching deeper path segments
                        let mut path = current_path.clone();
                        path.components.push(component.clone());
                        child.get_candidates(url_parts, candidates, idx + 1, path, method.clone());
                    }
                } else if let Some(f) = child.value.get(&method) {
                    // Reached the end of the URL, add handler if present
                    let mut path = current_path.clone();
                    path.components.push(component.clone());
                    candidates.push((path, f));
                }
            }
        }
    }
}
#[cfg(test)]
mod test {
    use http::Method;

    use crate::{
        handler::{HandlerTrie, types::HttpHandlerError},
        request::HttpRequest,
        response::HttpResponse,
    };

    /// Test handler that returns a default response.
    async fn test_handler(_: HttpRequest) -> Result<HttpResponse, HttpHandlerError> {
        Ok(HttpResponse::new())
    }

    /// Tests that route matching follows the correct precedence rules.
    ///
    /// This test verifies that when multiple patterns match a URL, the
    /// most specific one (according to Exact > PathParam > SingleSegmentWildcard > MultiSegmentWildcard)
    /// is selected.
    #[test]
    fn test_route_precedence() {
        let mut handler = HandlerTrie::default();

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
            "Route { components: [Exact(\"\"), Exact(\"url\"), PathParam(\"abc\"), PathParam(\"efg\")] }",
            format!("{:?}", matched_route)
        );
    }

    // #[test]
    // fn proxy_registers_common_http_methods() {
    //     let mut handlers = HandlerTrie::default();
    //     handlers.proxy("/api/**", "http://localhost:7799/api/v1");

    //     for method in [
    //         Method::GET,
    //         Method::POST,
    //         Method::PUT,
    //         Method::DELETE,
    //         Method::PATCH,
    //         Method::HEAD,
    //         Method::OPTIONS,
    //         Method::TRACE,
    //     ] {
    //         assert!(handlers.get_handler("/api/users", method).is_some());
    //     }
    // }
}
