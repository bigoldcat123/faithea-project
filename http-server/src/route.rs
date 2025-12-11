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
#[derive(Debug,Hash,PartialEq,Eq,Clone)]
pub enum RouteComponent {
    Exact(String),
    PathParam(String),
    SingleSegWilCard,
    MutiSegWildCard,
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
                RouteComponent::SingleSegWilCard => 2,
                RouteComponent::MutiSegWildCard => 1,
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
    pub fn match_url(&self,s:&str) -> bool {

        match self {
            Self::Exact(ss) => s == ss,
            _ => {
                true
            }
        }
    }
}
impl From<&str> for RouteComponent {
    fn from(value: &str) -> Self {
        if value.starts_with("{") {
            Self::PathParam(value[1..value.len() - 1].to_string())
        } else if value == "*" {
            Self::SingleSegWilCard
        } else if value == "**" {
            Self::MutiSegWildCard
        } else {
            Self::Exact(value.to_string())
        }
    }
}
#[derive(Debug,Clone,PartialEq, PartialOrd, Ord,Eq)]
pub struct Route {
    pub r: Vec<RouteComponent>,
}

impl TryFrom<&str> for Route {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut v = value.into();
        if !value.starts_with("/") {
            v = format!("/{}",value);
        }
        if v.ends_with("/") && v.len() != 1 {
            v.pop();
        }
        let mut r:Vec<RouteComponent> = vec![];
        for p in v.split("/") {
            r.push(p.into());
        }
        Ok(Self { r })
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create() {
        let r = "/hello/abc/*/{efg}/**";
        let a = Route::try_from(r).unwrap();
        println!("{:?}",a);

    }
}
