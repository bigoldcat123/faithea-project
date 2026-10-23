pub(crate) mod trie;
use std::{
    io::{self, ErrorKind},
    path::{Component, Path},
};

use http::StatusCode;

use crate::{
    data::outbound::MappedStaticFile,
    error::{BeforeHandlerError, Error},
    request::HttpRequest,
    res_modifiers,
    response::HttpResponseModifier,
};

/// # how to use
/// ``` ignore
/// use faithea::{get, util::static_map};
///
/// #[get("/**")]
/// pub async fn file_map() {
///     static_map(_req,"path/to/static/directory").await;
/// }
/// ```
/// Paths containing parent-directory or absolute components, and symlinks resolving
/// outside the mapped directory, are rejected with `403 Forbidden`.
pub async fn static_map<P: AsRef<str>>(
    _req: &HttpRequest,
    path: P,
) -> Vec<Box<dyn HttpResponseModifier + Send + Sync>> {
    if let Some(multi_seg_param) = _req.multi_seg_param.as_ref()
        && let Ok(multi_seg_param) = urlencoding::decode(multi_seg_param)
    {
        // /hello/something
        // 1. /hello/something
        // 2. /hello/something/index.html
        // 3. /hello/something.html
        //
        match resolve_static_file(Path::new(path.as_ref()), &multi_seg_param).await {
            Ok(Some(file)) => res_modifiers!(file),
            Ok(None) => res_modifiers!(Error::BeforeHandler(
                BeforeHandlerError::ParseHandlerParamError(
                    crate::request::error::ParseHandlerParamError::ParamNotExist,
                ),
            )),
            Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                res_modifiers!(StatusCode::FORBIDDEN, "forbidden")
            }
            Err(e) => res_modifiers!(Error::from(e)),
        }
    } else {
        res_modifiers!("no multi_seg_param found try to use /abc/** route!")
    }
}
async fn resolve_static_file(root: &Path, segment: &str) -> io::Result<Option<MappedStaticFile>> {
    // Validate after URL decoding, including encoded separators and Windows prefixes.
    if Path::new(segment).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(io::Error::from(ErrorKind::PermissionDenied));
    }

    let root = match tokio::fs::canonicalize(root).await {
        Ok(root) => root,
        Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    let exact = root.join(segment);
    let candidates = [
        exact.clone(),
        exact.join("index.html"),
        root.join(format!(
            "{}.html",
            segment.strip_suffix('/').unwrap_or(segment)
        )),
    ];
    for candidate in candidates {
        let resolved = match tokio::fs::canonicalize(&candidate).await {
            Ok(resolved) => resolved,
            Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {
                continue;
            }
            Err(e) => return Err(e),
        };
        // Path::starts_with compares components, so a sibling such as public-private
        // cannot pass a check against public. Check every fallback after symlinks resolve.
        if !resolved.starts_with(&root) {
            return Err(io::Error::from(ErrorKind::PermissionDenied));
        }
        if tokio::fs::metadata(&resolved).await?.is_file() {
            // Open the resolved path rather than following the original symlink again.
            return Ok(Some(MappedStaticFile {
                resolved_path: resolved,
                requested_path: candidate,
            }));
        }
    }
    Ok(None)
}
