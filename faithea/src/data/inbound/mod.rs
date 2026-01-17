pub mod multipart;
use std::{ops::{Deref, DerefMut}, sync::Arc};

use crate::{handler::types::HttpHandlerError, request::{HttpRequest, TryFromRequest}};
pub type Shared<T> = Arc<T>;


pub struct FromRequest<T>(T);

impl <T> FromRequest<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}
impl <T> Deref for FromRequest<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl <T> DerefMut for FromRequest<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl <'a,T:TryFromRequest<'a>> TryFromRequest<'a> for FromRequest<T> {
    fn try_from_request(req: &'a mut HttpRequest) -> Result<Self, HttpHandlerError> {
        let a:T = T::try_from_request(req)?;
        Ok(Self(a))
    }
}
