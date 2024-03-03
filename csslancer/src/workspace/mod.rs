pub mod source;

use anyhow::anyhow;
use lsp_types::{TextDocumentContentChangeEvent, Url};
use std::collections::HashMap;

use crate::config::PositionEncoding;
use crate::interop::LspRange;
use source::Source;

pub struct Workspace {
    pub files: HashMap<Url, Source>,
    // cache
    // id: FileId,
    // text: Prehashed<String>,
    // root: Prehashed<SyntaxNode>,
    // lines: Vec<Line>,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn open(&mut self, url: Url, doc: Source) {
        self.files.insert(url, doc);
    }

    pub fn close(&mut self, uri: &Url) {
        self.files.remove(uri);
    }

    pub fn clear(&mut self) {
        self.files.clear();
    }

    // pub fn get_document_cloned(&self, uri: &Url) -> FsResult<Source> {
    //     self.get_document_ref(uri).cloned()
    // }

    pub fn get_document_ref(&self, uri: &Url) -> FsResult<&Source> {
        self.files
            .get(uri)
            .ok_or_else(|| FsError::NotProvided(anyhow!("URI not found")))
    }

    pub fn get_document_mut(&mut self, uri: &Url) -> FsResult<&mut Source> {
        self.files
            .get_mut(uri)
            .ok_or_else(|| FsError::NotProvided(anyhow!("URI not found")))
    }

    pub fn register_files(&mut self) -> FsResult<()> {
        Ok(())
    }

    pub fn edit(
        &mut self,
        uri: &Url,
        changes: impl IntoIterator<Item = TextDocumentContentChangeEvent>,
        position_encoding: PositionEncoding,
    ) {
        let Ok(source) = self.get_document_mut(uri) else {
            return;
        };
        changes
            .into_iter()
            .for_each(|change| Self::apply_one_change(source, change, position_encoding));
    }

    fn apply_one_change(
        source: &mut Source,
        change: TextDocumentContentChangeEvent,
        position_encoding: PositionEncoding,
    ) {
        let replacement = change.text;

        match change.range {
            Some(lsp_range) => {
                let range = LspRange::new(lsp_range, position_encoding).into_range_on(source);
                source.edit(range, &replacement);
            }
            None => {
                source.replace(&replacement);
            }
        }
    }
}

pub type FsResult<T> = Result<T, FsError>;

#[derive(thiserror::Error, Debug)]
pub enum FsError {
    #[error("expected Csslancer source file, but found something else")]
    NotSource,
    #[error("could not find `{0}` on the local filesystem")]
    NotFoundLocal(std::path::PathBuf),
    // #[error(transparent)]
    // Package(#[from] PackageError),
    #[error(transparent)]
    OtherIo(std::io::Error),
    #[error("the provider does not provide the requested URI")]
    NotProvided(#[source] anyhow::Error),
    // #[error("could not join path to URI")]
    // UriJoin(#[from] UriError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
