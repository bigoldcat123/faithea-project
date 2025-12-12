//! HTTP request handler and routing system.
//!
//! This module provides the core routing infrastructure for the HTTP server,
//! including both simple handler maps and a sophisticated trie-based router
//! that supports complex route patterns.
//!
//! # Routing Types
//!
//! The router supports four types of route patterns:
//! 1. **Exact routes**: Match path segments exactly (e.g., `/api/users`)
//! 2. **Path parameters**: Capture segments as named parameters (e.g., `/api/users/{id}`)
//! 3. **Single-segment wildcards**: Match exactly one segment (e.g., `/api/*/details`)
//! 4. **Multi-segment wildcards**: Match any number of segments (e.g., `/api/**`)
//!
//! # Matching Precedence
//!
//! When multiple patterns match a request URL, they are prioritized as:
//! Exact > Path parameters > Single-segment wildcard > Multi-segment wildcard
//!
//! # Examples
//!
//! ```rust
//! use http_server::{HandlerTire, HttpRequest, HttpResponse};
//!
//! let mut router = HandlerTire::default();
//! router.add("/api/users", async |_| HttpResponse::new());
//! router.add("/api/users/{id}", async |_| HttpResponse::new());
//! router.add("/api/*/status", async |_| HttpResponse::new());
//! router.add("/static/**", async |_| HttpResponse::new());
//! ```

use std::{collections::HashMap, pin::Pin, future::Future};

use crate::{
    regulate_url_path, request::HttpRequest, response::HttpResponse, route::{Route, RouteComponent}
};

/// Type alias for a boxed async HTTP handler function.
///
/// A handler is an asynchronous function that takes an [`HttpRequest`] and
/// returns an [`HttpResponse`]. Handlers must be thread-safe (`Send + Sync`)
/// and have a static lifetime.
///
/// This type is used internally by both [`Handler`] and [`HandlerTire`].
pub type Fu = Box<
    dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = Result<HttpResponse,String>> + Send + Sync + 'static>>
        + Send
        + Sync
        + 'static,
>;

/// A prefix tree (trie) for efficient HTTP request routing with pattern matching.
///
/// `HandlerTire` organizes handlers in a trie structure based on their route
/// patterns, allowing for fast lookup of the most specific matching handler
/// for any given URL.
///
/// # Route Pattern Support
///
/// The trie supports four types of route patterns:
///
/// 1. **Exact routes** - Match path segments exactly (highest priority)
///    - Example: `/api/users`
///    - Created from literal path segments
///
/// 2. **Path parameters** - Capture segments as named parameters
///    - Example: `/api/users/{id}`
///    - Created from segments like `{param_name}`
///    - Matches any single segment and captures it as `param_name`
///
/// 3. **Single-segment wildcards** - Match exactly one arbitrary segment
///    - Example: `/api/*/details`
///    - Created from the `*` segment
///    - Matches any single segment but doesn't capture it
///
/// 4. **Multi-segment wildcards** - Match any number of remaining segments
///    - Example: `/api/**`
///    - Created from the `**` segment
///    - Matches zero or more segments (greedy match)
///
/// # Matching Precedence
///
/// When multiple patterns match a request URL, the handler is selected based on:
/// `Exact > Path parameters > Single-segment wildcard > Multi-segment wildcard`
///
/// Within the same category, longer/more specific paths are preferred.
///
/// # Performance
///
/// The trie structure provides O(k) lookup time where k is the number of
/// path segments in the URL, independent of the total number of registered routes.
///
/// # Examples
///
/// ```rust
/// use http_server::{HandlerTire, HttpRequest, HttpResponse};
///
/// let mut router = HandlerTire::default();
///
/// // Exact match for homepage
/// router.add("/", async |_| HttpResponse::new());
///
/// // Path parameter for user profiles
/// router.add("/users/{id}", async |req| {
///     println!("User ID from URL path");
///     HttpResponse::new()
/// });
///
/// // Wildcard for versioned API
/// router.add("/api/v*/*", async |_| {
///     println!("Any API version and endpoint");
///     HttpResponse::new()
/// });
///
/// // Catch-all for static files
/// router.add("/static/**", async |_| {
///     println("Static file request");
///     HttpResponse::new()
/// });
/// ```
#[derive(Default)]
pub struct HandlerTire {
    /// Child nodes in the routing trie, keyed by route components
    path: HashMap<RouteComponent, Box<Self>>,
    /// Handler function stored at this node (if this is a terminal node)
    f: HashMap<String,Fu>,
}
impl HandlerTire {
    pub fn mount(&mut self,modifiers:Vec<Box<dyn Fn(&mut Self)>>) {
        for m in modifiers {
            m(self);
        }
    }

    pub fn get<F, O, P>(&mut self, url: P, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpResponse,String>> + 'static + Send + Sync,
        P: AsRef<str>
    {
        let url = regulate_url_path(url);
        let mut route = Route::from(url.as_str());
        route.r.reverse();
        self.add_route(route.r,Box::new(move |r: HttpRequest| Box::pin(f(r))),"get");
    }
    pub fn post<F, O, P>(&mut self, url: P, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpResponse,String>> + 'static + Send + Sync,
        P: AsRef<str>
    {
        let url = regulate_url_path(url);
        let mut route = Route::from(url.as_str());
        route.r.reverse();
        self.add_route(route.r,Box::new(move |r: HttpRequest| Box::pin(f(r))),"post");
    }

    fn add_route(&mut self, mut url: Vec<RouteComponent>, f: Fu,method:&str)
    where
    {
        if let Some(next) = url.pop() {
            if !self.path.contains_key(&next) {
                self.path.insert(next.clone(), Default::default());
            }
            if url.is_empty() {
                self.path.get_mut(&next).unwrap().f.insert(method.to_string(), f);
            } else {
                self.path.get_mut(&next).unwrap().add_route(url, f,method);
            }
        }
    }

    pub fn get_handler(&self, url: &str,method:&str) -> Option<(Route, &Fu)> {
        let url = regulate_url_path(url);
        let url_parts: Vec<&str> = url.split("/").collect();
        let mut candidates: Vec<(Route, &Fu)> = vec![];
        self.get_candidates(&url_parts, &mut candidates, 0, Route { r: vec![] },method.to_lowercase().as_str());

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
        method:&str
    ) {
        if idx < url_parts.len() {
            let url_part = url_parts[idx];
            for (component, child) in self.path.iter().filter(|(comp, _)| comp.match_url(url_part)) {
                if idx + 1 < url_parts.len() {
                    if *component == RouteComponent::MultiSegWildCard {
                        // Multi-segment wildcard matches the rest of the path
                        if let Some(f) = child.f.get(method) {
                            let mut path = current_path.clone();
                            path.r.push(component.clone());
                            candidates.push((path, f));
                        }
                    } else {
                        // Continue matching deeper path segments
                        let mut path = current_path.clone();
                        path.r.push(component.clone());
                        child.get_candidates(url_parts, candidates, idx + 1, path,method);
                    }
                } else if let Some(f) = child.f.get(method) {
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
    use crate::{handler::HandlerTire, request::HttpRequest, response::HttpResponse};

    /// Test handler that returns a default response.
    async fn test_handler(_: HttpRequest) -> Result<HttpResponse,String> {
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
        let (matched_route, _) = handler.get_handler("/url/ab2c/efg","get").unwrap();

        // Should match the path parameter pattern, not the wildcards
        assert_eq!(
            "Route { r: [Exact(\"\"), Exact(\"url\"), PathParam(\"abc\"), PathParam(\"efg\")] }",
            format!("{:?}", matched_route)
        );
    }
}
