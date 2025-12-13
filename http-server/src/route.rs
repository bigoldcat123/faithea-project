#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum RouteComponent {
    Exact(String),

    PathParam(String),

    SingleSegWildCard,

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
    pub fn match_url(&self, s: &str) -> bool {
        match self {
            Self::Exact(ss) => s == ss,
            _ => true,
        }
    }
}
impl From<&str> for RouteComponent {
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

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Default)]
pub struct Route {
    /// The sequence of route components that make up this route pattern.
    pub r: Vec<RouteComponent>,
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
        let mut r: Vec<RouteComponent> = vec![];

        if let Some((url, _)) = v.split_once("?") {
            for p in url.split("/") {
                r.push(p.into());
            }
        } else {
            for p in v.split("/") {
                r.push(p.into());
            }
        }

        Self { r }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create() {
        use super::RouteComponent::*;
        let r = "/hello/abc/*/{efg}/**";
        let a = Route::try_from(r).unwrap();
        assert_eq!(
            Route {
                r: vec![
                    Exact("".to_string()),
                    Exact("hello".to_string()),
                    Exact("abc".to_string()),
                    SingleSegWildCard,
                    PathParam("efg".to_string()),
                    MultiSegWildCard
                ]
            },
            a
        );
    }
}
