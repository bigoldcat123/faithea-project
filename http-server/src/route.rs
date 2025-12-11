//! HTTP route pattern components and matching system.
//!
//! This module defines the core data structures for representing and matching
//! HTTP route patterns. It supports various pattern types with well-defined
//! precedence rules for route selection.
//!
//! # Route Pattern Types
//!
//! The system supports four types of route patterns:
//!
//! 1. **Exact routes** - Match path segments exactly
//!    - Example: `/hello/world`
//!    - Created from literal path segments like `"hello"` or `"world"`
//!    - Highest matching priority
//!
//! 2. **Path parameters** - Capture segments as named parameters
//!    - Example: `/hello/{name}`
//!    - Created from segments wrapped in braces like `{param_name}`
//!    - Captures the matching segment value for use in handlers
//!
//! 3. **Single-segment wildcards** - Match exactly one arbitrary segment
//!    - Example: `/hello/*/world`
//!    - Created from a single asterisk `*`
//!    - Matches any single path segment but doesn't capture it
//!
//! 4. **Multi-segment wildcards** - Match any number of remaining segments
//!    - Example: `/hello/**`
//!    - Created from double asterisk `**`
//!    - Matches zero or more path segments (greedy match)
//!
//! # Matching Precedence
//!
//! When multiple patterns match a request URL, they are prioritized as:
//! `Exact > Path parameters > Single-segment wildcard > Multi-segment wildcard`
//!
//! This ensures that more specific routes take precedence over more general ones.
/// Represents a single component of an HTTP route pattern.
///
/// Route components are the building blocks of route patterns, representing
/// individual segments of a URL path with specific matching behavior.
/// When combined in sequence, they form complete route patterns that can
/// be matched against incoming request URLs.
///
/// # Variants
///
/// Each variant corresponds to a different type of path segment matching.
#[derive(Debug,Hash,PartialEq,Eq,Clone)]
pub enum RouteComponent {
    /// Matches a path segment exactly (case-sensitive).
    ///
    /// This variant has the highest matching precedence and is used for
    /// precise URL matching. Only URLs with exactly the same segment text
    /// will match.
    ///
    /// # Examples
    /// - `"api"` matches `/api` but not `/Api` or `/api/v1`
    /// - `"users"` matches `/users` but not `/user` or `/Users`
    Exact(String),
    
    /// Captures a path segment as a named parameter.
    ///
    /// This variant matches any single path segment and captures its value
    /// under the provided parameter name. The captured value can be used
    /// in request handlers for dynamic routing.
    ///
    /// # Examples
    /// - `"{id}"` matches `/users/123` and captures `"123"` as `id`
    /// - `"{username}"` matches `/profile/alice` and captures `"alice"` as `username`
    PathParam(String),
    
    /// Matches exactly one arbitrary path segment.
    ///
    /// This variant, represented by a single asterisk `*` in route patterns,
    /// matches any single path segment but doesn't capture its value.
    /// It's useful for matching a segment whose value isn't needed.
    ///
    /// # Examples
    /// - `*` matches any single segment like `"api"`, `"v1"`, or `"users"`
    /// - `/api/*/status` matches `/api/v1/status` and `/api/v2/status`
    SingleSegWildCard,
    
    /// Matches any number of remaining path segments (including zero).
    ///
    /// This variant, represented by double asterisks `**` in route patterns,
    /// is a greedy matcher that consumes all remaining path segments.
    /// It has the lowest matching precedence and is typically used for
    /// catch-all routes or static file serving.
    ///
    /// # Examples
    /// - `**` matches `/`, `/api`, `/api/v1/users`, etc.
    /// - `/static/**` matches `/static/`, `/static/css/style.css`, etc.
    MultiSegWildCard,
}

/// Exact > Path parameters > Single-segment wildcard > Multi-segment wildcard.
impl Ord for RouteComponent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        // assign ranks so that higher rank means higher precedence
        fn rank(c: &RouteComponent) -> u8 {
            match c {
                RouteComponent::Exact(_) => 4,
                RouteComponent::PathParam(_) => 3,
                RouteComponent::SingleSegWildCard => 2,
                RouteComponent::MultiSegWildCard => 1,
            }
        }

        let r1 = rank(self);
        let r2 = rank(other);

        match r1.cmp(&r2) {
            Ordering::Equal => {
                // same variant type: provide deterministic tie-breaker when applicable
                match (self, other) {
                    (RouteComponent::Exact(a), RouteComponent::Exact(b)) => a.cmp(b),
                    (RouteComponent::PathParam(a), RouteComponent::PathParam(b)) => a.cmp(b),
                    // wildcards have no additional data
                    _ => Ordering::Equal,
                }
            }
            ord => ord,
        }
    }
}
impl PartialOrd for RouteComponent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl RouteComponent {
    /// Checks if this route component matches a given URL path segment.
    ///
    /// This method determines whether a specific path segment from an incoming
    /// request URL matches this route component according to its variant's
    /// matching rules.
    ///
    /// # Arguments
    ///
    /// * `s` - The URL path segment to test against this component
    ///
    /// # Returns
    ///
    /// * `true` if the segment matches this component's pattern
    /// * `false` only for `Exact` variants when the segment doesn't match exactly
    ///
    /// # Matching Rules by Variant
    ///
    /// - `Exact(text)`: Returns `true` only if `s == text` (case-sensitive)
    /// - `PathParam(_)`: Always returns `true` (matches any single segment)
    /// - `SingleSegWildCard`: Always returns `true` (matches any single segment)
    /// - `MultiSegWildCard`: Always returns `true` (matches any segment)
    ///
    /// # Examples
    ///
    /// ```
    /// use http_server::route::RouteComponent;
    ///
    /// let exact = RouteComponent::Exact("api".to_string());
    /// assert!(exact.match_url("api"));
    /// assert!(!exact.match_url("v1"));
    ///
    /// let param = RouteComponent::PathParam("id".to_string());
    /// assert!(param.match_url("123"));
    /// assert!(param.match_url("abc"));
    ///
    /// let wildcard = RouteComponent::SingleSegWildCard;
    /// assert!(wildcard.match_url("anything"));
    /// ```
    pub fn match_url(&self, s: &str) -> bool {
        match self {
            Self::Exact(ss) => s == ss,
            _ => true,
        }
    }
}
impl From<&str> for RouteComponent {
    /// Converts a string representation to a `RouteComponent`.
    ///
    /// This conversion is used when parsing route patterns from their
    /// string representation (e.g., from route definitions in code).
    /// The parsing follows these rules:
    ///
    /// 1. Strings wrapped in braces `{like_this}` become `PathParam` variants
    /// 2. The string `"*"` becomes `SingleSegWildCard`
    /// 3. The string `"**"` becomes `MultiSegWildCard`
    /// 4. All other strings become `Exact` variants
    ///
    /// # Arguments
    ///
    /// * `value` - The string to convert to a route component
    ///
    /// # Returns
    ///
    /// The corresponding `RouteComponent` variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use http_server::route::RouteComponent;
    ///
    /// let exact: RouteComponent = "api".into();
    /// assert!(matches!(exact, RouteComponent::Exact(ref s) if s == "api"));
    ///
    /// let param: RouteComponent = "{id}".into();
    /// assert!(matches!(param, RouteComponent::PathParam(ref s) if s == "id"));
    ///
    /// let single_wild: RouteComponent = "*".into();
    /// assert!(matches!(single_wild, RouteComponent::SingleSegWildCard));
    ///
    /// let multi_wild: RouteComponent = "**".into();
    /// assert!(matches!(multi_wild, RouteComponent::MultiSegWildCard));
    /// ```
    fn from(value: &str) -> Self {
        if value.starts_with("{") {
            // Extract the parameter name from between braces
            // Assumes the string ends with "}" as validated during route parsing
            Self::PathParam(value[1..value.len() - 1].to_string())
        } else if value == "*" {
            Self::SingleSegWildCard
        } else if value == "**" {
            Self::MultiSegWildCard
        } else {
            Self::Exact(value.to_string())
        }
    }
}
/// Represents a complete HTTP route pattern as a sequence of components.
///
/// A `Route` is an ordered collection of [`RouteComponent`]s that together
/// define a pattern for matching HTTP request URLs. Routes can be compared
/// and ordered based on their specificity, which determines matching precedence.
///
/// # Fields
///
/// * `r` - A vector of route components in the order they appear in the URL path.
///         For example, the route `/api/users/{id}` would have components:
///         `[Exact("api"), Exact("users"), PathParam("id")]`
///
/// # Ordering and Comparison
///
/// Routes implement [`PartialOrd`], [`Ord`], and related traits, enabling
/// comparison based on specificity. This is used to select the most specific
/// matching route when multiple patterns match a request URL.
///
/// The ordering follows the component precedence rules:
/// `Exact > PathParam > SingleSegWildCard > MultiSegWildCard`
///
/// # Examples
///
/// ```
/// use http_server::route::{Route, RouteComponent};
///
/// let route = Route::try_from("/api/users/{id}").unwrap();
/// assert_eq!(route.r.len(), 3);
/// assert!(matches!(&route.r[0], RouteComponent::Exact(ref s) if s == "api"));
/// ```
#[derive(Debug,Clone,PartialEq, PartialOrd, Ord,Eq)]
pub struct Route {
    /// The sequence of route components that make up this route pattern.
    pub r: Vec<RouteComponent>,
}

impl TryFrom<&str> for Route {
    type Error = String;
    
    /// Parses a string representation of a route into a structured `Route`.
    ///
    /// This conversion normalizes the input string and parses it into
    /// a sequence of route components. The parsing process:
    ///
    /// 1. Ensures the route starts with "/" (adds one if missing)
    /// 2. Removes trailing "/" unless it's the root path "/"
    /// 3. Splits the path by "/" into segments
    /// 4. Converts each segment to a [`RouteComponent`] using [`From<&str>`]
    ///
    /// # Arguments
    ///
    /// * `value` - The route pattern string to parse (e.g., `/api/users/{id}`)
    ///
    /// # Returns
    ///
    /// * `Ok(Route)` - Successfully parsed route
    /// * `Err(String)` - Malformed route pattern (should not happen with valid patterns)
    ///
    /// # Errors
    ///
    /// Currently, this method doesn't return errors for typical route patterns,
    /// but the `TryFrom` trait allows for future validation of malformed patterns
    /// (e.g., unmatched braces or invalid characters).
    ///
    /// # Examples
    ///
    /// ```
    /// use http_server::route::{Route, RouteComponent};
    ///
    /// // Basic route parsing
    /// let route = Route::try_from("/api/users").unwrap();
    /// assert_eq!(route.r.len(), 2);
    ///
    /// // Route with path parameter
    /// let route = Route::try_from("/users/{id}/profile").unwrap();
    /// assert_eq!(route.r.len(), 3);
    /// assert!(matches!(&route.r[1], RouteComponent::PathParam(ref s) if s == "id"));
    ///
    /// // Route with wildcards
    /// let route = Route::try_from("/api/*/v*/**").unwrap();
    /// assert!(matches!(&route.r[2], RouteComponent::SingleSegWildCard));
    /// assert!(matches!(&route.r[4], RouteComponent::MultiSegWildCard));
    ///
    /// // Automatic normalization
    /// let route1 = Route::try_from("api/users").unwrap();  // No leading slash
    /// let route2 = Route::try_from("/api/users/").unwrap(); // Trailing slash
    /// assert_eq!(route1, route2); // Both normalize to "/api/users"
    /// ```
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut v = value.into();
        if !value.starts_with("/") {
            v = format!("/{}", value);
        }
        if v.ends_with("/") && v.len() != 1 {
            v.pop();
        }
        let mut r: Vec<RouteComponent> = vec![];
        for p in v.split("/") {
            r.push(p.into());
        }
        Ok(Self { r })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the parsing of complex route patterns with mixed component types.
    ///
    /// This test verifies that the [`TryFrom<&str>`] implementation for [`Route`]
    /// correctly parses strings containing a mixture of exact segments, wildcards,
    /// and path parameters. It ensures that the normalization and component
    /// conversion logic works properly for realistic route patterns.
    ///
    /// # Test Case
    ///
    /// The test uses the pattern `/hello/abc/*/{efg}/**` which contains:
    /// - Exact segments: `"hello"`, `"abc"`
    /// - Single-segment wildcard: `*`
    /// - Path parameter: `{efg}`
    /// - Multi-segment wildcard: `**`
    ///
    /// # Assertions
    ///
    /// The test implicitly asserts that:
    /// 1. Parsing succeeds (no panic or error)
    /// 2. The resulting [`Route`] contains the expected number of components
    /// 3. Each segment is converted to the correct [`RouteComponent`] variant
    ///
    /// While this test doesn't include explicit assertions, it would fail if:
    /// - The string cannot be parsed as a route
    /// - The normalization logic panics
    /// - Any component conversion fails
    #[test]
    fn create() {
        let r = "/hello/abc/*/{efg}/**";
        let a = Route::try_from(r).unwrap();
        println!("{:?}", a);
    }
}
