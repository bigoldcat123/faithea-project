use std::collections::HashMap;

use crate::route::Route;


#[derive(Debug, Default)]
pub struct PathParam {
    pub(crate) _inner: HashMap<String, String>,
}
impl PathParam {
    pub fn get<S: AsRef<str>>(&self, key: S) -> Option<&String> {
        self._inner.get(key.as_ref())
    }
    pub(crate) fn try_from_route(handler_route: &Route, incoming_route: &Route) -> Result<Self, String> {
        use crate::route::RouteComponent::*;
        let mut _inner = HashMap::new();
        if handler_route.r.len() != incoming_route.r.len() {
            return Err("route len not match!".to_string());
        }
        for cmp in handler_route.r.iter().zip(incoming_route.r.iter()) {
            match cmp {
                (PathParam(p), Exact(v)) => {
                    _inner.insert(p.to_string(), v.to_string());
                }
                _ => {}
            }
        }
        if _inner.is_empty() {
            Err("no path params".to_string())
        } else {
            Ok(Self { _inner })
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_single_param_parsing_test() {
        let handler_route = Route::from("/hello/{name}");
        let incoming_route = Route::from("/hello/chenzhonghai");
        let p = PathParam::try_from_route(&handler_route, &incoming_route).unwrap();
        let a = p.get("name").unwrap();
        assert_eq!(a, "chenzhonghai")
    }
    #[test]
    fn path_multi_param_parsing_test() {
        let handler_route = Route::from("/hello/{name}/{age}/dadigua");
        let incoming_route = Route::from("/hello/chenzhonghai/22/dadigua");
        let p = PathParam::try_from_route(&handler_route, &incoming_route).unwrap();
        let a = p.get("name").unwrap();
        let age = p.get("age").unwrap();
        assert_eq!(a, "chenzhonghai");
        assert_eq!(age, "22");
    }

}
