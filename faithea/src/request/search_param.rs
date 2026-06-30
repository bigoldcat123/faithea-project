use std::collections::HashMap;

#[derive(Debug, Default)]
pub(crate) struct SearchParam {
    pub(crate) _inner: HashMap<String, String>,
}

impl SearchParam {
    pub(crate) fn from_query(query: Option<&str>) -> Self {
        let mut map = HashMap::new();
        if let Some(search_params) = query {
            for (k, v) in search_params.split("&").filter_map(|x| x.split_once("=")) {
                if let Ok(ok) = urlencoding::decode(v) {
                    map.insert(k.into(), ok.to_string());
                }
            }
        }
        Self { _inner: map }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 快速构造 HashMap 的宏，减少视觉噪音
    macro_rules! map {
        ($( $k:expr => $v:expr ),* $(,)?) => {
            HashMap::from([
                $( ($k.into(), $v.into()) ),*
            ])
        };
    }

    #[test]
    fn spaces_around_query() {
        let url = "  x=1  & y = 2  ";
        let s = SearchParam::from_query(Some(url));
        // 空格保留在 value 里，取决于你的 trim 策略，这里假设不自动 trim
        assert_eq!(s._inner, map! { "  x" => "1  ", " y " => " 2  " });
    }

    #[test]
    fn given_weird_case() {
        // 题目自带的用例
        let url = "a=a=10&c=200 ";
        let s = SearchParam::from_query(Some(url));
        assert_eq!(s._inner, map! { "a" => "a=10", "c" => "200 " });
    }
    #[test]
    fn search_param_test() {
        let url = "";
        let s = SearchParam::from_query(Some(url));
        assert!(s._inner.is_empty())
    }
    #[test]
    fn search_param_test2() {
        let url = "a=a=10&c=200";
        let s = SearchParam::from_query(Some(url));
        assert_eq!(
            s._inner,
            HashMap::from([("a".into(), "a=10".into()), ("c".into(), "200".into())])
        )
    }

    #[test]
    fn basic_kv() {
        let url = "name=kimi&age=18";
        let s = SearchParam::from_query(Some(url));
        assert_eq!(s._inner, map! { "name" => "kimi", "age" => "18" });
    }

    #[test]
    fn empty_value() {
        let url = "key=";
        let s = SearchParam::from_query(Some(url));
        assert_eq!(s._inner, map! { "key" => "" });
    }

    #[test]
    fn empty_key() {
        let url = "=value";
        let s = SearchParam::from_query(Some(url));
        // 空字符串当 key 也是合法实现，这里按「空 key」处理
        assert_eq!(s._inner, map! { "" => "value" });
    }

    #[test]
    fn duplicate_keys_keep_last() {
        let url = "a=1&b=2&a=3";
        let s = SearchParam::from_query(Some(url));
        assert_eq!(s._inner, map! { "a" => "3", "b" => "2" });
    }

    #[test]
    fn no_query_string() {
        let url = "";
        let s = SearchParam::from_query(Some(url));
        assert_eq!(s._inner, map! {});
    }

    #[test]
    fn question_mark_only() {
        let url = "";
        let s = SearchParam::from_query(Some(url));
        assert_eq!(s._inner, map! {});
    }
}
