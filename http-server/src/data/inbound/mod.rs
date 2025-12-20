pub mod multipart;
use std::{ops::{Deref, DerefMut}, sync::Arc};

use crate::{handler::FuError, request::HttpRequest};
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
// deprecate
impl <'a,T:TryFrom<&'a HttpRequest> + 'a> TryFrom<&'a HttpRequest> for FromRequest<T> {
    type Error = FuError;
    fn try_from(value: &'a HttpRequest) -> Result<Self, Self::Error> {
        let a:T = value.try_into().map_err(|_| Box::new("can not covert httpReques  to T") as Self::Error)?;
        Ok(Self(a))
    }
}

impl <'a,T:TryFrom::<&'a mut HttpRequest> + 'a> TryFrom<&'a mut HttpRequest> for FromRequest<T> {
    type Error = FuError;
    fn try_from(value: &'a mut HttpRequest) -> Result<Self, Self::Error> {
        let a:T = value.try_into().map_err(|_| Box::new("can not covert httpReques  to T") as Self::Error)?;
        Ok(Self(a))
    }
}
