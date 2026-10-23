#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum RouteComponent {
    Exact(String),

    PathParam(String),

    SingleSegmentWildcard,

    MultiSegmentWildcard,
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
                RouteComponent::SingleSegmentWildcard => 2,
                RouteComponent::MultiSegmentWildcard => 1,
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
    pub fn match_url(&self, s: &str) -> bool {
        match self {
            Self::Exact(ss) => s == ss,
            _ => true,
        }
    }
}
impl From<&str> for RouteComponent {
    fn from(value: &str) -> Self {
        // Keep malformed brace expressions literal because this conversion is infallible.
        if let Some(name) = value.strip_prefix('{').and_then(|v| v.strip_suffix('}'))
            && !name.is_empty()
            && !name.contains(['{', '}'])
        {
            Self::PathParam(name.to_string())
        } else if value == "*" {
            Self::SingleSegmentWildcard
        } else if value == "**" {
            Self::MultiSegmentWildcard
        } else {
            Self::Exact(value.to_string())
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Default)]
pub struct Route {
    /// The sequence of route components that make up this route pattern.
    pub components: Vec<RouteComponent>,
}

impl From<&str> for Route {
    fn from(value: &str) -> Self {
        let mut v = value.into();
        if !value.starts_with("/") {
            v = format!("/{}", value);
        }
        if v.ends_with("/") && v.len() != 1 {
            v.pop();
        }
        let mut components: Vec<RouteComponent> = vec![];

        if let Some((url, _)) = v.split_once("?") {
            for p in url.split("/") {
                components.push(p.into());
            }
        } else {
            for p in v.split("/") {
                components.push(p.into());
            }
        }

        Self { components }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_parameters_are_literal_segments() {
        for segment in ["{", "}", "{}", "{x", "x}", "{{x}}", "{x}y}", "{中", "{😀"] {
            let route = Route::from(format!("/users/{segment}").as_str());
            assert_eq!(
                route.components.last(),
                Some(&RouteComponent::Exact(segment.to_string())),
                "segment: {segment:?}"
            );
        }
    }

    #[test]
    fn valid_parameters_preserve_the_entire_name() {
        for name in ["x", "id", "用户名", "😀"] {
            assert_eq!(
                RouteComponent::from(format!("{{{name}}}").as_str()),
                RouteComponent::PathParam(name.to_string())
            );
        }
    }

    #[test]
    fn create() {
        use super::RouteComponent::*;
        let pattern = "/hello/abc/*/{efg}/**";
        let route = Route::from(pattern);
        assert_eq!(
            Route {
                components: vec![
                    Exact("".to_string()),
                    Exact("hello".to_string()),
                    Exact("abc".to_string()),
                    SingleSegmentWildcard,
                    PathParam("efg".to_string()),
                    MultiSegmentWildcard
                ]
            },
            route
        );
    }
}
