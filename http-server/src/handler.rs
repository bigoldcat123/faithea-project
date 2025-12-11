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
    dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = HttpResponse> + Send + Sync + 'static>>
        + Send
        + Sync
        + 'static,
>;

/// A simple map-based HTTP request handler.
///
/// This structure provides basic URL-to-handler mapping using exact string
/// matching. It's suitable for small, simple applications or testing, but
/// for production use with complex routing patterns, prefer [`HandlerTire`].
///
/// # Examples
///
/// ```rust
/// use http_server::{Handler, HttpRequest, HttpResponse};
///
/// let mut handler = Handler::default();
/// handler.add("/hello".to_string(), async |_| {
///     let mut resp = HttpResponse::new();
///     resp.add_header(("Content-Type", "text/plain"));
///     resp
/// });
/// ```
#[derive(Default)]
pub struct Handler {
    handles: HashMap<String, Fu>,
}
impl Handler {
    /// Registers a handler function for a specific URL path.
    ///
    /// The handler will be called for requests whose URL exactly matches
    /// the provided string. No pattern matching is performed.
    ///
    /// # Arguments
    ///
    /// * `url` - The exact URL path to match (e.g., "/api/users")
    /// * `f` - The handler function to call for matching requests
    ///
    /// # Type Parameters
    ///
    /// * `F` - The handler function type
    /// * `O` - The future returned by the handler function
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::{Handler, HttpRequest, HttpResponse};
    ///
    /// let mut handler = Handler::default();
    /// handler.add("/home".to_string(), async |req: HttpRequest| {
    ///     println!("Received request for: {}", req.req_line.url);
    ///     HttpResponse::new()
    /// });
    /// ```
    pub fn add<F, O>(&mut self, url: String, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = HttpResponse> + 'static + Send + Sync,
    {
        let a: Fu = Box::new(move |r: HttpRequest| Box::pin(f(r)));
        self.handles.insert(url, a);
    }
    
    /// Looks up a handler for the given URL path.
    ///
    /// Returns `None` if no handler is registered for the exact URL.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL path to look up
    ///
    /// # Returns
    ///
    /// * `Some(&Fu)` - Reference to the handler function if found
    /// * `None` - No handler registered for this exact URL
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::{Handler, HttpRequest, HttpResponse};
    ///
    /// let mut handler = Handler::default();
    /// handler.add("/test".to_string(), async |_| HttpResponse::new());
    ///
    /// assert!(handler.get("/test").is_some());
    /// assert!(handler.get("/other").is_none());
    /// ```
    pub fn get(&self, url: &str) -> Option<&Fu> {
        self.handles.get(url)
    }
}
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
    f: Option<Fu>,
}
impl HandlerTire {
    /// Registers a handler function for the specified route pattern.
    ///
    /// The handler will be called for any request whose URL matches the pattern.
    /// If multiple patterns match the same URL, the most specific one (according
    /// to the precedence rules) will be selected.
    ///
    /// # Arguments
    ///
    /// * `url` - The route pattern to match (any type implementing `AsRef<str>`)
    /// * `f` - The handler function to register
    ///
    /// # Type Parameters
    ///
    /// * `F` - The handler function type
    /// * `O` - The future returned by the handler function
    /// * `P` - The URL pattern type (must implement `AsRef<str>`)
    ///
    /// # Panics
    ///
    /// Panics if the URL pattern cannot be parsed as a valid route.
    /// This typically indicates a malformed pattern (e.g., unmatched braces).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::{HandlerTire, HttpRequest, HttpResponse};
    ///
    /// let mut router = HandlerTire::default();
    ///
    /// // Exact route
    /// router.add("/api/users", async |_| HttpResponse::new());
    ///
    /// // Route with path parameter
    /// router.add("/api/users/{id}", async |req| {
    ///     // The {id} segment will be captured
    ///     HttpResponse::new()
    /// });
    ///
    /// // Route with wildcard
    /// router.add("/api/*/status", async |_| HttpResponse::new());
    /// ```
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
    /// Internal helper to recursively add a handler to the trie.
    ///
    /// This method traverses the trie, creating nodes as needed, and stores
    /// the handler function at the terminal node corresponding to the complete
    /// route pattern.
    ///
    /// # Arguments
    ///
    /// * `url` - The route pattern decomposed into components (in reverse order)
    /// * `f` - The handler function to register
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
    /// Finds the most specific handler matching the given URL.
    ///
    /// This method searches the trie for all handlers whose patterns match
    /// the URL, then selects the most specific one according to the
    /// precedence rules.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL path to match against registered route patterns
    ///
    /// # Returns
    ///
    /// * `Some((Route, &Fu))` - The matched route pattern and handler function
    /// * `None` - No handler matches the URL (404 case)
    ///
    /// The returned [`Route`] contains the actual pattern that matched, which
    /// can be useful for debugging or for extracting path parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::{HandlerTire, HttpRequest, HttpResponse};
    ///
    /// let mut router = HandlerTire::default();
    /// router.add("/api/users", async |_| HttpResponse::new());
    /// router.add("/api/{resource}", async |_| HttpResponse::new());
    /// router.add("/api/**", async |_| HttpResponse::new());
    ///
    /// // Exact match takes precedence
    /// let (matched_route, handler) = router.get("/api/users").unwrap();
    /// assert!(format!("{:?}", matched_route).contains("Exact"));
    ///
    /// // No match returns None
    /// assert!(router.get("/other/path").is_none());
    /// ```
    pub fn get(&self, url: &str) -> Option<(Route, &Fu)> {
        let url = regulate_url_path(url);
        let url_parts: Vec<&str> = url.split("/").collect();
        let mut candidates: Vec<(Route, &Fu)> = vec![];
        self.get_candidates(&url_parts, &mut candidates, 0, Route { r: vec![] });

        candidates.sort_by(|a, b| a.0.cmp(&b.0));
        // Debug logging (commented out in production):
        // for route in candidates.iter().map(|x| &x.0) {
        //     println!("Candidate route: {:?}", route);
        // }

        candidates.pop()
    }
    /// Recursively collects candidate handlers that match the URL.
    ///
    /// This is the core matching algorithm that traverses the trie to find
    /// all handlers whose patterns match the given URL path segments.
    /// Multi-segment wildcards (`**`) match greedily and terminate the
    /// recursion at their position.
    ///
    /// # Arguments
    ///
    /// * `url_parts` - The URL split into path segments
    /// * `candidates` - Mutable vector to collect matching handlers
    /// * `idx` - Current index in the URL parts being matched
    /// * `current_path` - The route path built so far during traversal
    fn get_candidates<'a>(
        &'a self,
        url_parts: &Vec<&str>,
        candidates: &mut Vec<(Route, &'a Fu)>,
        idx: usize,
        current_path: Route,
    ) {
        if idx < url_parts.len() {
            let url_part = url_parts[idx];
            for (component, child) in self.path.iter().filter(|(comp, _)| comp.match_url(url_part)) {
                if idx + 1 < url_parts.len() {
                    if *component == RouteComponent::MultiSegWildCard {
                        // Multi-segment wildcard matches the rest of the path
                        if let Some(f) = child.f.as_ref() {
                            let mut path = current_path.clone();
                            path.r.push(component.clone());
                            candidates.push((path, f));
                        }
                    } else {
                        // Continue matching deeper path segments
                        let mut path = current_path.clone();
                        path.r.push(component.clone());
                        child.get_candidates(url_parts, candidates, idx + 1, path);
                    }
                } else if let Some(f) = child.f.as_ref() {
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
    async fn test_handler(_: HttpRequest) -> HttpResponse {
        HttpResponse::new()
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
        handler.add("/url/abc/efg", test_handler);
        handler.add("/url/{abc}/{efg}", test_handler);
        handler.add("/url/abc", test_handler);
        handler.add("/url/*/efg", test_handler);
        handler.add("/url/**", test_handler);
        
        // Test URL that matches multiple patterns
        let (matched_route, _) = handler.get("/url/ab2c/efg").unwrap();
        
        // Should match the path parameter pattern, not the wildcards
        assert_eq!(
            "Route { r: [Exact(\"\"), Exact(\"url\"), PathParam(\"abc\"), PathParam(\"efg\")] }",
            format!("{:?}", matched_route)
        );
    }
}
