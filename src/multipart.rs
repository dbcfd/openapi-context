//! Helper functions for multipart support

use headers::{ContentType, HeaderMap, HeaderMapExt};
use mime;

/// Utility function to get the multipart boundary marker (if any) from the Headers.
pub fn boundary(headers: &HeaderMap) -> Option<String> {
    headers.typed_get::<ContentType>().and_then(|content_type| {
        if mime.type_() == mime::MULTIPART && mime.subtype() == mime::FORM_DATA {
            mime.get_param(mime::BOUNDARY).map(|x| x.to_string())
        } else {
            None
        }
    })
}
