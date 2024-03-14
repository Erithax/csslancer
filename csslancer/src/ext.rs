use std::path::Path;

use ego_tree::{NodeId, Tree};
use tower_lsp::lsp_types::{DocumentFormattingClientCapabilities, Url};
use tower_lsp::lsp_types::{
    InitializeParams, Position, PositionEncodingKind, SemanticTokensClientCapabilities,
};

use crate::config::PositionEncoding;

pub struct LocalFs {}

impl LocalFs {
    /// Convert a path to its corresponding `file://` URI. Returns `Err` if the path is not
    /// absolute.
    pub fn path_to_uri(path: impl AsRef<Path>) -> Result<Url, FsPathToUriError> {
        Url::from_file_path(path).map_err(|()| FsPathToUriError::NotAbsolute)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum FsPathToUriError {
    #[error("cannot convert to URI since path is not absolute")]
    NotAbsolute,
}

pub trait InitializeParamsExt {
    fn position_encodings(&self) -> &[PositionEncodingKind];
    fn supports_config_change_registration(&self) -> bool;
    fn semantic_tokens_capabilities(&self) -> Option<&SemanticTokensClientCapabilities>;
    fn document_formatting_capabilities(&self) -> Option<&DocumentFormattingClientCapabilities>;
    fn supports_semantic_tokens_dynamic_registration(&self) -> bool;
    fn supports_document_formatting_dynamic_registration(&self) -> bool;
    fn root_uris(&self) -> Vec<Url>;
}

static DEFAULT_ENCODING: [PositionEncodingKind; 1] = [PositionEncodingKind::UTF16];

impl InitializeParamsExt for InitializeParams {
    fn position_encodings(&self) -> &[PositionEncodingKind] {
        self.capabilities
            .general
            .as_ref()
            .and_then(|general| general.position_encodings.as_ref())
            .map(|encodings| encodings.as_slice())
            .unwrap_or(&DEFAULT_ENCODING)
    }

    fn supports_config_change_registration(&self) -> bool {
        self.capabilities
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.configuration)
            .unwrap_or(false)
    }

    fn semantic_tokens_capabilities(&self) -> Option<&SemanticTokensClientCapabilities> {
        self.capabilities
            .text_document
            .as_ref()?
            .semantic_tokens
            .as_ref()
    }

    fn document_formatting_capabilities(&self) -> Option<&DocumentFormattingClientCapabilities> {
        self.capabilities
            .text_document
            .as_ref()?
            .formatting
            .as_ref()
    }

    fn supports_semantic_tokens_dynamic_registration(&self) -> bool {
        self.semantic_tokens_capabilities()
            .and_then(|semantic_tokens| semantic_tokens.dynamic_registration)
            .unwrap_or(false)
    }

    fn supports_document_formatting_dynamic_registration(&self) -> bool {
        self.document_formatting_capabilities()
            .and_then(|document_format| document_format.dynamic_registration)
            .unwrap_or(false)
    }

    #[allow(deprecated)] // `self.root_path` is marked as deprecated
    fn root_uris(&self) -> Vec<Url> {
        match self.workspace_folders.as_ref() {
            Some(roots) => roots.iter().map(|root| &root.uri).cloned().collect(),
            None => {
                let root_uri = || self.root_uri.as_ref().cloned();
                let root_path = || LocalFs::path_to_uri(self.root_path.as_ref()?).ok();

                root_uri().or_else(root_path).into_iter().collect()
            }
        }
    }
}

pub trait StrExt {
    fn encoded_len(&self, encoding: PositionEncoding) -> usize;
}

impl StrExt for str {
    fn encoded_len(&self, encoding: PositionEncoding) -> usize {
        match encoding {
            PositionEncoding::Utf8 => self.len(),
            PositionEncoding::Utf16 => self.chars().map(char::len_utf16).sum(),
        }
    }
}

pub trait PathExt {
    fn is_csslancer(&self) -> bool;
}

impl PathExt for Path {
    fn is_csslancer(&self) -> bool {
        self.extension().map_or(false, |ext| ext == "csslancer")
    }
}

// pub trait VirtualPathExt {
//     fn with_extension(&self, extension: impl AsRef<OsStr>) -> Self;
// }

// impl VirtualPathExt for VirtualPath {
//     fn with_extension(&self, extension: impl AsRef<OsStr>) -> Self {
//         Self::new(self.as_rooted_path().with_extension(extension))
//     }
// }

// pub trait FileIdExt {
//     fn with_extension(self, extension: impl AsRef<OsStr>) -> Self;
//     fn fill(self, current: PackageId) -> FullFileId;
// }

// impl FileIdExt for FileId {
//     fn with_extension(self, extension: impl AsRef<OsStr>) -> Self {
//         let path = self.vpath().with_extension(extension);
//         Self::new(self.package().cloned(), path)
//     }

//     fn fill(self, current: PackageId) -> FullFileId {
//         let package = self
//             .package()
//             .cloned()
//             .map(PackageId::new_external)
//             .unwrap_or(current);
//         FullFileId::new(package, self.vpath().clone())
//     }
// }

pub trait PositionExt {
    fn delta(&self, to: &Self) -> PositionDelta;
}

impl PositionExt for Position {
    /// Calculates the delta from `self` to `to`. This is in the `SemanticToken` sense, so the
    /// delta's `character` is relative to `self`'s `character` iff `self` and `to` are on the same
    /// line. Otherwise, it's relative to the start of the line `to` is on.
    fn delta(&self, to: &Self) -> PositionDelta {
        assert!(
            to.line >= self.line,
            "tried to get token delta from a token to token on a earlier line"
        );
        let line_delta = to.line - self.line;
        assert!(
            line_delta != 0 || to.character >= self.character,
            "tried to get token delta from a token to a token earlier in the line"
        );
        let char_delta = if line_delta == 0 {
            to.character - self.character
        } else {
            to.character
        };

        PositionDelta {
            delta_line: line_delta,
            delta_start: char_delta,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Default)]
pub struct PositionDelta {
    pub delta_line: u32,
    pub delta_start: u32,
}

pub trait TreeAttach {
    fn attach_tree(&mut self, other: &mut Self, self_at: NodeId, other_at: NodeId);
}

impl<T: Default> TreeAttach for Tree<T> {
    fn attach_tree(&mut self, other: &mut Self, self_at: NodeId, other_at: NodeId) {
        let mut self_node = self.get_mut(self_at).unwrap();
        let new_id = self_node.append(std::mem::take(other.get_mut(other_at).unwrap().value())).id();
        let ch_ids: Vec<NodeId> = other.root().children().map(|ch| ch.id()).collect();

        for ch_id in ch_ids {
            self.attach_tree(other, new_id, ch_id);
        }
    }
}


//pub trait UrlExt {
/// Joins the path to the URI, treating the URI as if it was the root directory. Returns `Err`
/// if the path leads out of the root or the URI cannot be used as a base.
//fn join_rooted(self, vpath: &VirtualPath) -> UriResult<Url>;

/// Gets the relative path to the sub URI, treating this URI as if it was the root. Returns
/// `None` if the path leads out of the root.
//fn make_relative_rooted(&self, sub_uri: &Url) -> UriResult<VirtualPath>;

/// Unless this URL is cannot-be-a-base, returns the path segments, percent decoded into UTF-8
/// strings, if possible.
//fn path_segments_decoded(&self) -> UriResult<Vec<Cow<str>>>;

/// Get a new URI, replacing the existing file extension with the given extension, if there is a
/// file extension to replace.
//fn with_extension(self, extension: &str) -> UriResult<Url>;
//}

//impl UrlExt for Url {
// fn join_rooted(mut self, vpath: &VirtualPath) -> Result<Url, UriError> {
//     let mut added_len: usize = 0;
//     let mut segments = self
//         .path_segments_mut()
//         .map_err(|()| UriError::CannotBeABase)?;

//     for component in vpath.as_rootless_path().components() {
//         match component {
//             Component::Normal(segment) => {
//                 added_len += 1;
//                 segments.push(segment.to_str().expect("all package paths should be UTF-8"));
//             }
//             Component::ParentDir => {
//                 added_len.checked_sub(1).ok_or(UriError::PathEscapesRoot)?;
//                 segments.pop();
//             }
//             Component::CurDir => (),
//             // should occur only at the start, when the URI is already root, so nothing to do
//             Component::Prefix(_) | Component::RootDir => (),
//         }
//     }

//     // must drop before return to ensure its `Drop` doesn't use borrowed `self` after move
//     drop(segments);

//     Ok(self)
// }

// fn make_relative_rooted(&self, sub_uri: &Url) -> UriResult<VirtualPath> {
//     if self.scheme() != sub_uri.scheme() || self.authority() != sub_uri.authority() {
//         return Err(UriError::PathEscapesRoot);
//     }

//     let root = self.path_segments_decoded()?;
//     let sub = sub_uri.path_segments_decoded()?;

//     let root_iter = root.iter().map(Cow::as_ref);
//     let sub_iter = sub.iter().map(Cow::as_ref);

//     let relative_path: PathBuf = root_iter
//         .zip_longest(sub_iter)
//         .skip_while(|x| matches!(x, EitherOrBoth::Both(left, right) if left == right))
//         .map(|x| x.just_right().ok_or(UriError::PathEscapesRoot))
//         .try_collect()?;

//     Ok(VirtualPath::new(relative_path))
// }

// fn path_segments_decoded(&self) -> UriResult<Vec<Cow<str>>> {
//     self.path_segments()
//         .ok_or(UriError::CannotBeABase)
//         .and_then(|segments| {
//             segments
//                 .map(percent_decode_str)
//                 .map(PercentDecode::decode_utf8)
//                 .try_collect()
//                 .map_err(UriError::from)
//         })
// }

// fn with_extension(mut self, extension: &str) -> UriResult<Url> {
//     let filename = self
//         .path_segments()
//         .ok_or(UriError::CannotBeABase)?
//         .last()
//         .unwrap_or("");
//     let filename_decoded = percent_decode_str(filename).decode_utf8()?;

//     let new_filename_path = Path::new(filename_decoded.as_ref()).with_extension(extension);
//     let new_filename = new_filename_path
//         .to_str()
//         .expect("the path should come from `filename` and `extension`; both are valid UTF-8");

//     self.path_segments_mut()
//         .map_err(|()| UriError::CannotBeABase)?
//         .pop()
//         .push(new_filename);

//     Ok(self)
// }
//}

// pub type UriResult<T> = Result<T, UriError>;

// #[derive(thiserror::Error, Debug, PartialEq, Eq)]
// pub enum UriError {
//     #[error("URI cannot be a base")]
//     CannotBeABase,
//     #[error("path escapes root")]
//     PathEscapesRoot,
//     #[error("could not decode")]
//     Encoding(#[from] Utf8Error),
// }

#[cfg(test)]
mod uri_test {
    //use super::*;

    // #[test]
    // fn join_rooted() {
    //     let url = Url::parse("file:///path/to").unwrap();
    //     let path = VirtualPath::new("/file.typ");

    //     let joined = url.join_rooted(&path).unwrap();

    //     let expected = Url::parse("file:///path/to/file.typ").unwrap();
    //     assert_eq!(expected, joined);
    // }

    // #[test]
    // fn join_rooted_utf8() {
    //     let url = Url::parse("file:///path/%E6%B1%89%E5%AD%97/to").unwrap();
    //     let path = VirtualPath::new("/汉字.typ");

    //     let joined = url.join_rooted(&path).unwrap();

    //     let expected =
    //         Url::parse("file:///path/%E6%B1%89%E5%AD%97/to/%E6%B1%89%E5%AD%97.typ").unwrap();
    //     assert_eq!(expected, joined);
    // }

    // #[test]
    // fn join_rooted_escape() {
    //     let url = Url::parse("file:///path/to").unwrap();
    //     let escapee = VirtualPath::new("/../../etc/passwd");

    //     let error = url.join_rooted(&escapee).unwrap_err();

    //     assert_eq!(UriError::PathEscapesRoot, error);
    // }

    // #[test]
    // fn make_relative_rooted() {
    //     let base_url = Url::parse("file:///path").unwrap();
    //     let sub_url = Url::parse("file:///path/to/file.typ").unwrap();

    //     let relative = base_url.make_relative_rooted(&sub_url).unwrap();

    //     assert_eq!(VirtualPath::new("/to/file.typ"), relative);
    // }

    // #[test]
    // fn make_relative_rooted_utf8() {
    //     let base_url = Url::parse("file:///path/%E6%B1%89%E5%AD%97/dir").unwrap();
    //     let sub_url =
    //         Url::parse("file:///path/%E6%B1%89%E5%AD%97/dir/to/%E6%B1%89%E5%AD%97.typ").unwrap();

    //     let relative = base_url.make_relative_rooted(&sub_url).unwrap();

    //     assert_eq!(VirtualPath::new("/to/汉字.typ"), relative);
    // }

    // #[test]
    // fn make_relative_rooted_not_relative() {
    //     let base_url = Url::parse("file:///path/to").unwrap();
    //     let sub_url = Url::parse("file:///path/not/to/file.typ").unwrap();

    //     let err = base_url.make_relative_rooted(&sub_url).unwrap_err();

    //     assert_eq!(UriError::PathEscapesRoot, err)
    // }

    // #[test]
    // fn path_segments_decode() {
    //     let url = Url::parse("file:///path/to/file.typ").unwrap();

    //     let segments = url.path_segments_decoded().unwrap();

    //     assert_eq!(
    //         vec!["path", "to", "file.typ"],
    //         segments.iter().map(Cow::as_ref).collect_vec()
    //     )
    // }

    // #[test]
    // fn path_segments_decode_utf8() {
    //     let url = Url::parse("file:///path/to/file/%E6%B1%89%E5%AD%97.typ").unwrap();

    //     let segments = url.path_segments_decoded().unwrap();

    //     assert_eq!(
    //         vec!["path", "to", "file", "汉字.typ"],
    //         segments.iter().map(Cow::as_ref).collect_vec()
    //     )
    // }

    // #[test]
    // fn with_extension() {
    //     let url = Url::parse("file:///path/to/file.typ").unwrap();

    //     let pdf_url = url.with_extension("pdf").unwrap();

    //     let expected = Url::parse("file:///path/to/file.pdf").unwrap();
    //     assert_eq!(expected, pdf_url);
    // }

    // #[test]
    // fn with_extension_utf8() {
    //     let url = Url::parse("file:///path/to/file/%E6%B1%89%E5%AD%97.typ").unwrap();

    //     let pdf_url = url.with_extension("pdf").unwrap();

    //     let expected = Url::parse("file:///path/to/file/%E6%B1%89%E5%AD%97.pdf").unwrap();
    //     assert_eq!(expected, pdf_url);
    // }
}
