use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use http::{HeaderValue, header::SET_COOKIE};

use crate::{handler::types::HttpHandlerError, response::HttpResponseModifier};

pub enum CookieType {
    KeyValue(String, String),
    Attribute(String),
}
impl Debug for CookieType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CookieType::Attribute(attr) => {
                write!(f, "{attr};")
            }
            CookieType::KeyValue(k, v) => {
                write!(f, "{k}={v};")
            }
        }
    }
}
#[derive(Default)]
pub struct Cookie {
    _innser: Vec<CookieType>,
}

impl Debug for Cookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in &self._innser {
            write!(f, "{:?}", c)?;
        }
        Ok(())
    }
}

impl Deref for Cookie {
    type Target = Vec<CookieType>;
    fn deref(&self) -> &Self::Target {
        &self._innser
    }
}
impl DerefMut for Cookie {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self._innser
    }
}

impl HttpResponseModifier for Cookie {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut super::HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), HttpHandlerError>> + 'a + Send + Sync>>
    {
        Box::pin(async move {
            res._innser.headers_mut().insert(
                SET_COOKIE,
                HeaderValue::from_maybe_shared(format!("{:?}", self))?,
            );
            Ok(())
        })
    }
}
