use std::{collections::HashMap, ops::{Deref, DerefMut}};

use crate::{handler::FuError, response::HttpResponseModifier};

#[derive(Debug, Default)]
pub struct Cookie {
    _innser: HashMap<String, String>,
}

impl Deref for Cookie {
    type Target = HashMap<String,String>;
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
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), FuError>> + 'a + Send + Sync>> {
        Box::pin(async move {
            for (k,v) in self._innser.drain() {
                res.headers.add(k,v);
            }
            Ok(())
        })
    }
}
