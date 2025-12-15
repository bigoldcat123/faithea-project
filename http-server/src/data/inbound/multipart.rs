use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    usize,
};

use bytes::Bytes;

use crate::{map_str, request::HttpRequest};

pub trait TryFromMultipartDataMap: Sized {
    fn try_from(data: &mut HashMap<String, Part>) -> Result<Self, String>;
}

#[derive(Debug)]
pub enum Part {
    Lit(String),
    File {
        file_name: Option<String>,
        data: Bytes,
        mime_type: Option<String>,
    },
}
#[derive(Debug)]
pub struct MultiPartFile {
    pub file_name: Option<String>,
    pub data: Bytes,
    pub mime_type: Option<String>,
}

impl TryFrom<Part> for MultiPartFile {
    type Error = String;
    fn try_from(value: Part) -> Result<Self, Self::Error> {
        if let Part::File {
            file_name,
            data,
            mime_type,
        } = value
        {
            Ok(Self {
                file_name,
                data,
                mime_type,
            })
        } else {
            Err("not compatiable to transform part to MultiPartFile".to_string())
        }
    }
}








#[derive(Debug)]
pub struct Multipart<T: TryFromMultipartDataMap>(T);

impl<T: TryFromMultipartDataMap> Multipart<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: TryFromMultipartDataMap> Deref for Multipart<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: TryFromMultipartDataMap> DerefMut for Multipart<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a,T: TryFromMultipartDataMap> TryFrom<&HttpRequest> for Multipart<T> {
    type Error = String;
    fn try_from(req: &HttpRequest) -> Result<Self, Self::Error> {
        match (&req.body, get_multipart_boundary(req)) {
            (Some(body), Some(boundary)) => {
                let mut data = HashMap::new();
                parse_multipart_to_map(&body[boundary.len() + 2..], boundary.as_bytes(), &mut data);
                return Ok(Multipart(T::try_from(&mut data)?));
            }
            _ => return Err("no boundary".into()),
        }
    }
}

fn parse_multipart_to_map(b: &[u8], boundary: &[u8], data: &mut HashMap<String, Part>) {
    let mut r = 0;
    let mut l = 0;
    while r < b.len() {
        // search headers for /r/n
        let mut name = "";
        let mut file_name = None;
        let mut mime_type = None;
        while r < b.len() {
            while r + 2 < b.len() && &b[r..r + 2] != b"\r\n" {
                r += 1;
            }
            if l == r {
                r += 2;
                l = r;
                break;
            }
            process_multipart_header(&b[l..r], &mut name, &mut file_name, &mut mime_type);
            r += 2;
            l = r;
        }
        //search for body /r/n +  boundary
        while r + 2 < b.len()
            && r + 2 + boundary.len() < b.len()
            && !(&b[r..r + 2] == b"\r\n" && &b[r + 2..r + 2 + boundary.len()] == boundary)
        {
            r += 1;
        }
        //the file part
        if file_name.is_some() || mime_type.is_some() {
            data.insert(
                name.to_string(),
                Part::File {
                    file_name,
                    data: Bytes::copy_from_slice(&b[l..r]),
                    mime_type,
                },
            );
        } else {
            // the lit part
            data.insert(
                name.to_string(),
                Part::Lit(String::from_utf8_lossy(&b[l..r]).to_string()),
            );
        }
        // bytes end with `--`
        if &b[r + boundary.len() + 2..r + 2 + boundary.len() + 2] == b"--" {
            break;
        }
        r += 2 + boundary.len() + 2;
        l = r;
    }
}

fn process_multipart_header<'a>(
    header_line: &'a [u8],
    name: &mut &'a str,
    file_name: &mut Option<String>,
    mime_type: &mut Option<String>,
) {
    if let Ok(h) = str::from_utf8(header_line) {
        if let Some((k, v)) = h.split_once(":") {
            if k.eq_ignore_ascii_case("Content-Disposition") {
                for kv in v.split(";") {
                    if let Some((k, v)) = kv.split_once("=") {
                        if k.trim() == "name" {
                            *name = &v[1..v.len() - 1];
                        }
                        if k.trim() == "filename" {
                            *file_name = Some(v[1..v.len() - 1].to_string())
                        }
                    }
                }
            } else if k.eq_ignore_ascii_case("Content-Type") {
                *mime_type = Some(v.trim().to_string())
            }
        }
    }
}

fn get_multipart_boundary(req: &HttpRequest) -> Option<String> {
    if let Some(b) = req.headers.get("content-type") {
        if let Some((_, b)) = b.split_once(";") {
            if let Some((_, b)) = b.split_once("=") {
                return Some(format!("--{}", b));
            }
        }
    }
    None
}
macro_rules! impl_try_from_part_for_parse_from_str {
    ($($t:ty),*) => {
        $(
            impl TryFrom<Part> for $t {
                type Error = String;
                fn try_from(value: Part) -> Result<Self, Self::Error> {
                    if let Part::Lit(l) = value {
                        Ok(l.parse::<Self>().map_err(map_str!())?)
                    }else {
                        Err("not compatiable to transform part to MultiPartFile".to_string())
                    }
                }
            }
        )*
    };
}
impl_try_from_part_for_parse_from_str!(
    i8, i16, i32, i64, i128, isize, usize, f32, f64, u8, u16, u32, u64, u128, bool, String
);
// #[cfg(test)]
// mod tests {
//     use super::*;
//     // struct Stu {
//     //     name: String,
//     //     age: u8,
//     //     profile: Bytes,
//     // }
//     // impl Stu {

//     // }
//     #[test]
//     fn test_multipart() {}
// }
