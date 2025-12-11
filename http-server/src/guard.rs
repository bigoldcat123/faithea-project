//! Guard middleware system for HTTP request validation and filtering.
//!
//! This module provides a guard system that allows asynchronous validation
//! and transformation of HTTP requests before they reach handlers.
//! Guards are similar to middleware in other frameworks and can be used
//! for authentication, logging, rate limiting, or any pre-processing logic.
//!
//! # Key Concepts
//!
//! - **Guard**: An asynchronous function that takes a request and returns either
//!   a modified request or an error response.
//! - **Guard Trie**: A prefix tree that organizes guards by their route patterns,
//!   allowing efficient matching based on URL paths.
//! - **Guard Chain**: When multiple guards match a request URL, they form a chain
//!   that executes in order of specificity (most specific first).
//!
//! # Usage
//!
//! ```rust
//! use http_server::{GuardTire, HttpRequest, HttpResponse};
//!
//! let mut guards = GuardTire::default();
//! guards.add("/api/*", async |req: HttpRequest| {
//!     // Validate authentication
//!     Ok(req) // Pass request to next guard or handler
//! });
//! guards.add("/api/admin/**", async |req: HttpRequest| {
//!     // Check admin privileges
//!     Ok(req)
//! });
//! ```
//!
//! Guards execute in order from most specific to least specific route pattern.
//! If any guard returns an error response, the chain stops and the error is
//! returned to the client.

use std::{collections::HashMap, future::Future, pin::Pin};

use crate::{
    regulate_url_path, request::HttpRequest, response::HttpResponse, route::{Route, RouteComponent}
};

/// Type alias for a guard function.
///
/// A guard is an asynchronous function that takes an `HttpRequest` and returns
/// either a modified `HttpRequest` (to continue processing) or an `HttpResponse`
/// (to immediately respond with an error or redirect).
///
/// Guards must be thread-safe (`Send + Sync`) and have a static lifetime.
pub type Guard = Box<
    dyn Fn(
            HttpRequest,
        ) -> Pin<
            Box<dyn Future<Output = Result<HttpRequest, HttpResponse>> + Send + Sync + 'static>,
        > + Send
        + Sync
        + 'static,
>;

/// A prefix tree (trie) for organizing and matching guards by route patterns.
///
/// The `GuardTire` efficiently stores guards based on their route patterns
/// and can quickly find all guards that match a given URL path. When a request
/// arrives, all matching guards are collected into a chain and executed in
/// order from most specific to least specific.
///
/// # Route Pattern Matching
///
/// Guards support the same route patterns as handlers:
/// - Exact matches: `/api/users`
/// - Path parameters: `/api/users/{id}`
/// - Single-segment wildcards: `/api/*/details`
/// - Multi-segment wildcards: `/api/**`
///
/// # Examples
///
/// ```rust
/// use http_server::{GuardTire, HttpRequest, HttpResponse};
///
/// let mut guards = GuardTire::default();
/// guards.add("/api/**", async |req| {
///     println!("All API requests");
///     Ok(req)
/// });
/// guards.add("/api/users/*", async |req| {
///     println!("User-specific requests");
///     Ok(req)
/// });
/// ```
#[derive(Default)]
pub struct GuardTire {
    /// Child nodes in the trie, keyed by route components
    path: HashMap<RouteComponent, Box<Self>>,
    /// Guard function stored at this node (if this is a terminal node)
    f: Option<Guard>,
}

impl GuardTire {
    /// Registers a new guard function for the specified route pattern.
    ///
    /// The guard will be called for any request whose URL matches the pattern.
    /// If multiple guards match the same request, they will be executed in
    /// order from most specific to least specific pattern.
    ///
    /// # Arguments
    ///
    /// * `url` - The route pattern to match (any type implementing `AsRef<str>`)
    /// * `f` - The guard function to register
    ///
    /// # Type Parameters
    ///
    /// * `F` - The guard function type
    /// * `O` - The future returned by the guard function
    /// * `P` - The URL pattern type (must implement `AsRef<str>`)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::{GuardTire, HttpRequest, HttpResponse};
    ///
    /// let mut guards = GuardTire::default();
    /// guards.add("/secure/*", async |req: HttpRequest| {
    ///     // Check authentication
    ///     Ok(req)
    /// });
    /// ```
    pub fn add<F, O, P>(&mut self, url: P, f: F)
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpRequest, HttpResponse>> + 'static + Send + Sync,
        P: AsRef<str>
    {
        let url = regulate_url_path(url);
        let mut url_route = Route::try_from(url.as_str()).unwrap();
        url_route.r.reverse();
        self.add_url(url_route, f);
    }

    /// Internal helper to recursively add a guard to the trie.
    ///
    /// # Arguments
    ///
    /// * `url` - The route pattern decomposed into components (in reverse order)
    /// * `f` - The guard function to register
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

    /// Executes all guards that match the given URL for the provided request.
    ///
    /// This method finds all guards whose route patterns match the URL,
    /// orders them from most specific to least specific, then executes them
    /// sequentially. If any guard returns an error response, execution stops
    /// and the error is returned. Otherwise, the (potentially modified) request
    /// is returned for further processing.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL path to match against guard patterns
    /// * `req` - The HTTP request to process through the guard chain
    ///
    /// # Returns
    ///
    /// * `Ok(HttpRequest)` - The (potentially modified) request after all
    ///   guards have executed successfully
    /// * `Err(HttpResponse)` - An error response from the first guard that
    ///   rejected the request
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::{GuardTire, HttpRequest, HttpResponse};
    ///
    /// # async fn example() {
    /// let mut guards = GuardTire::default();
    /// guards.add("/api/**", async |req| Ok(req));
    ///
    /// let request = HttpRequest::new(/* ... */);
    /// match guards.guard("/api/users/123", request).await {
    ///     Ok(processed_req) => { /* Continue to handler */ }
    ///     Err(error_resp) => { /* Return error to client */ }
    /// }
    /// # }
    /// ```
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

    /// Builds the guard chain for a given URL.
    ///
    /// This method finds all guards that match the URL and returns them
    /// in a vector, sorted from most specific to least specific route pattern.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL path to match against guard patterns
    ///
    /// # Returns
    ///
    /// A vector of tuples containing the matched route and a reference to
    /// the guard function, sorted by specificity.
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

    /// Recursively collects candidate guards that match the URL.
    ///
    /// This is the core matching algorithm that traverses the trie to find
    /// all guards whose patterns match the given URL path segments.
    ///
    /// # Arguments
    ///
    /// * `url_parts` - The URL split into path segments
    /// * `candidates` - Mutable vector to collect matching guards
    /// * `idx` - Current index in the URL parts being matched
    /// * `current_path` - The route path built so far during traversal
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

    /// Tests that guard chain building correctly identifies matching guards
    /// in the proper order (most specific first).
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
