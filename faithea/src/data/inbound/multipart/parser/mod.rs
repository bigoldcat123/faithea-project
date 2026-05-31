pub mod h1;

#[derive(Debug, Clone, Copy)]
pub(crate) enum MultiPartBodyParserState {
    Start,
    Header,
    Body,
    End,
}
#[derive(Default, Debug)]
pub(crate) struct HeaderInfo {
    name: Option<String>,
    mime_type: Option<String>,
    file_name: Option<String>,
}
fn build_boundary_next_array(boundary: &[u8]) -> Vec<usize> {
    let mut ans = vec![0; boundary.len()];
    let mut i = 1;
    let mut len = 0;
    while i < boundary.len() - 1 {
        if boundary[i] == boundary[len] {
            len += 1;
            ans[i] = len;
            i += 1;
        } else if len > 0 {
            len = ans[len - 1];
        } else {
            i += 1;
        }
    }
    ans
}
