use std::{future::Future, pin::Pin};

use crate::{
    normalize_url_path,
    request::HttpRequest,
    response::HttpResponse,
    route::{Route, RouteComponent},
    util::trie::Trie,
};
type GuardResult = Result<HttpRequest, HttpResponse>;
pub trait GuardFuture: Future<Output = GuardResult> + Send + 'static {}
impl<T: Future<Output = GuardResult> + Send + 'static> GuardFuture for T {}
pub trait GuardTrait: Fn(HttpRequest) -> Pin<Box<dyn GuardFuture>> + Send + Sync + 'static {}
impl<T: Fn(HttpRequest) -> Pin<Box<dyn GuardFuture>> + Send + Sync + 'static> GuardTrait for T {}
pub trait RawGuardTrait<R: GuardFuture>: Fn(HttpRequest) -> R + Send + Sync + 'static {}
impl<T: Fn(HttpRequest) -> R + Send + Sync + 'static, R: GuardFuture> RawGuardTrait<R> for T {}
pub type Guard = Box<dyn GuardTrait>;
pub type GuardTrie = Trie<Vec<Guard>, RouteComponent>;
// #[derive(Default)]
// pub struct GuardTrie {
//     /// Child nodes in the trie, keyed by route components
//     path: HashMap<RouteComponent, Box<Self>>,
//     /// Guard function stored at this node (if this is a terminal node)
//     f: Vec<Guard>,
// }

impl GuardTrie {
    pub fn add<F, R, P>(&mut self, url: P, f: F)
    where
        F: RawGuardTrait<R>,
        R: GuardFuture,
        P: AsRef<str>,
    {
        let url = normalize_url_path(url);
        let mut url_route = Route::from(url.as_str());
        url_route.components.reverse();
        self.add_with_route_components(url_route, f);
    }

    fn add_with_route_components<F, R>(&mut self, mut url: Route, f: F)
    where
        F: RawGuardTrait<R>,
        R: GuardFuture,
    {
        if let Some(next) = url.components.pop() {
            if !self.children.contains_key(&next) {
                self.children.insert(next.clone(), Default::default());
            }
            if url.components.is_empty() {
                self.children
                    .get_mut(&next)
                    .unwrap()
                    .value
                    .push(Box::new(move |request: HttpRequest| Box::pin(f(request))))
            } else {
                self.children
                    .get_mut(&next)
                    .unwrap()
                    .add_with_route_components(url, f);
            }
        }
    }

    pub async fn guard(&self, url: &str, req: HttpRequest) -> Result<HttpRequest, HttpResponse> {
        let url = normalize_url_path(url);
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

    fn get_guard_chain(&self, url: &str) -> Vec<(Route, &Guard)> {
        let url_parts: Vec<&str> = url.split("/").collect();
        let mut candidates: Vec<(Route, &Guard)> = vec![];
        self.get_candidates(&url_parts, &mut candidates, 0, Route { components: vec![] });
        candidates.sort_by(|a, b| a.0.cmp(&b.0).reverse());

        // Debug logging (commented out in production)
        // for route in candidates.iter().map(|x| &x.0) {
        //     println!("-> {:?}", route);
        // }

        candidates
    }

    fn get_candidates<'a>(
        &'a self,
        url_parts: &Vec<&str>,
        candidates: &mut Vec<(Route, &'a Guard)>,
        idx: usize,
        current_path: Route,
    ) {
        if idx < url_parts.len() {
            let url_part = url_parts[idx];
            for (component, child) in self
                .children
                .iter()
                .filter(|(comp, _)| comp.match_url(url_part))
            {
                // Debug logging (commented out in production)
                // println!("{}, {:?}", url_part, component);

                if *component == RouteComponent::MultiSegmentWildcard {
                    // Multi-segment wildcard matches the rest of the path
                    let mut path = current_path.clone();
                    path.components.push(component.clone());
                    for g in child.value.iter() {
                        candidates.push((path.clone(), g));
                    }
                } else if idx + 1 < url_parts.len() {
                    // Continue matching deeper path segments
                    let mut path = current_path.clone();
                    path.components.push(component.clone());
                    child.get_candidates(url_parts, candidates, idx + 1, path);
                } else {
                    let mut path = current_path.clone();
                    path.components.push(component.clone());
                    for g in child.value.iter() {
                        candidates.push((path.clone(), g));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::guard::GuardTrie;

    #[test]
    fn test_guard_chain_ordering() {
        let mut guards = GuardTrie::default();
        guards.add("/url/*/efg", async |e| Ok(e));
        guards.add("/url/abc/efg", async |e| Ok(e));
        guards.add("/url/**", async |e| Ok(e));
        guards.add("/url/abc", async |e| Ok(e));

        let chain = guards.get_guard_chain("/url/abc/efg");
        let routes: Vec<_> = chain.iter().map(|x| &x.0).collect();
        assert_eq!(
            r#"[Route { components: [Exact(""), Exact("url"), Exact("abc"), Exact("efg")] }, Route { components: [Exact(""), Exact("url"), SingleSegmentWildcard, Exact("efg")] }, Route { components: [Exact(""), Exact("url"), MultiSegmentWildcard] }]"#,
            format!("{:?}", routes)
        );
    }
    #[test]
    fn test_guard_chain_ordering2() {
        let mut guards = GuardTrie::default();
        guards.add("/url/*/efg", async |e| Ok(e));
        guards.add("/url/abc/efg", async |e| Ok(e));
        guards.add("/url/abc/efg", async |e| Ok(e));
        guards.add("/url/**", async |e| Ok(e));
        guards.add("/url/abc", async |e| Ok(e));

        let chain = guards.get_guard_chain("/url/abc/efg");
        let routes: Vec<_> = chain.iter().map(|x| &x.0).collect();

        assert_eq!(routes.len(), 4);
    }
}
