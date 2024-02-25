
pub mod css_validation;

use std::sync::OnceLock;
use tokio::sync::RwLock;
use tracing_subscriber::{reload, Registry};
use lsp_types::*;
use serde_json::Value as JsonValue;
use lsp_types::{CompletionParams, CompletionResponse, DiagnosticOptions, DiagnosticServerCapabilities, DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams, DocumentSymbolResponse, ExecuteCommandParams, Hover, HoverParams, InitializeResult, ServerCapabilities, SignatureHelp, SignatureHelpParams, WorkDoneProgressOptions};
use tower_lsp::{Client, LanguageServer, jsonrpc::{self, Error}, lsp_types::{InitializeParams, Url}};
use crate::workspace::FsError;
use crate::workspace::source::Source;
use crate::logging::LspLayer;


use crate::workspace::Workspace;
use crate::config::ConstConfig;

pub struct CssLancerServer {
    pub client: Client,
    workspace: OnceLock<RwLock<Workspace>>,
    const_config: OnceLock<ConstConfig>,
    pub lsp_tracing_layer_handle: reload::Handle<Option<LspLayer>, Registry>,
}

impl CssLancerServer {
    pub fn new(client: Client, lsp_tracing_layer_handle: reload::Handle<Option<LspLayer>, Registry>) -> Self {
        return Self {
            workspace: Default::default(),
            const_config: Default::default(),
            client,
            lsp_tracing_layer_handle
        }
    }

    pub async fn workspace_read(&self) -> tokio::sync::RwLockReadGuard<'_, Workspace> {
        self.workspace
            .get()
            .expect("workspace should be initialized")
            .read().await
    }

    pub async fn workspace_write(&self) -> tokio::sync::RwLockWriteGuard<'_, Workspace> {
        self.workspace
            .get()
            .expect("workspace should be initialized")
            .write().await
    }

    #[tracing::instrument(skip(self))]
    pub async fn source_read(&self, url: &Url) -> Result<tokio::sync::RwLockReadGuard<'_, Source>, FsError> {
        // TODO: do this without double HashMap access (get_document_ref(url))
        let w = self.workspace_read().await;
        
        {
            let src: &Source = w.get_document_ref(url)?;
        }

        return Ok(tokio::sync::RwLockReadGuard::map(w, |w| w.get_document_ref(url).unwrap()));
    }

    pub fn const_config(&self) -> &ConstConfig {
        return self.const_config.get()
            .expect("const config should be initialized")
    }

    #[tracing::instrument(skip_all)]
    pub async fn on_source_changed(&self, uri: &Url) -> anyhow::Result<()> {
        //self.workspace_write().await.get_document_mut(uri).unwrap().
        self.publish_diags(uri).await.map_err(anyhow::Error::from)
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for CssLancerServer {

    #[tracing::instrument(skip(self))]
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        self.tracing_init();


        self.workspace
            .set(RwLock::new(Workspace::new()))
            .map_err(|_| ())
            .expect("workspace should not yet be initialized");

        self.const_config
            .set(ConstConfig::from(&params))
            .expect("const config should not yet be initialized");
        
        if let Err(err) = self.workspace_write().await.register_files() {
            tracing::error!(%err, "could not register workspace files on init");
            return Err(jsonrpc::Error::internal_error());
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                        ..Default::default()
                    },
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("csslancer".to_string()),
                        //identifier: None,
                        inter_file_dependencies: true,
                        workspace_diagnostics: true,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: None
                        }
                    }
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    #[tracing::instrument(skip_all)]
    async fn initialized(&self, _: InitializedParams) {
        tracing::trace!("INITIALIZED");
    }

    #[tracing::instrument(skip_all)]
    async fn shutdown(&self) -> jsonrpc::Result<()> {
        tracing::trace!("SHUTDOWN");
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client.log_message(MessageType::WARNING, "opening doc").await;
        tracing::warn!("did_open()");
        let doc = params.text_document;
        let url = doc.uri;
        let src = Source::new(url.clone(), doc.text, doc.version);

        let mut workspace = self.workspace_write().await;
        workspace.open(url.clone(), src);
        drop(workspace);

        if let Err(err) = self.on_source_changed(&url).await {
            tracing::error!(%err, %url, "could not handle source change");
        };
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        let mut workspace = self.workspace_write().await;
        workspace.close(&uri);
        drop(workspace);

        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        tracing::warn!("did_change()");
        self.client.log_message(MessageType::ERROR, "did_change()").await;
        let uri = params.text_document.uri;
        let changes = params.content_changes;
        let mut workspace = self.workspace_write().await;
        workspace.edit(&uri, changes, self.const_config().position_encoding);
        drop(workspace);

        if let Err(err) = self.on_source_changed(&uri).await {
            tracing::error!(%err, %uri, "could not handle source change");
        };
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        tracing::warn!("did_save()");
        self.client.log_message(MessageType::ERROR, "did_save()").await;
        let uri = params.text_document.uri;

        // if let Err(err) = self.run_diagnostics_and_export(&uri).await {
        //     tracing::error!(%err, %uri, "could not handle source save");
        // };
        if let Err(err) = self.on_source_changed(&uri).await {
            tracing::error!(%err, %uri, "could not handle source save");
        }
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> jsonrpc::Result<Option<JsonValue>> {
        Err(Error::invalid_request())
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> jsonrpc::Result<Option<CompletionResponse>> {
        Ok(None)
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> jsonrpc::Result<Option<SignatureHelp>> {
        Ok(None)
    }

    async fn document_symbol(&self, params: DocumentSymbolParams) -> jsonrpc::Result<Option<DocumentSymbolResponse>> {
        Ok(None)
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<Vec<SymbolInformation>>> {
        Ok(None)
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        Ok(None)
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> jsonrpc::Result<Option<SemanticTokensFullDeltaResult>> {
        Ok(None)
    }

    #[tracing::instrument(skip(self))]
    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {

    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> jsonrpc::Result<Option<Vec<SelectionRange>>> {
        Ok(None)
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        Ok(None)
    }
}


// pub struct WorldThread<'a> {
//     main: Source,
//     main_project: Project,
//     typst_thread: &'a TypstThread,
// }

// impl<'a> WorldThread<'a> {
//     pub async fn run<T: Send + 'static>(
//         self,
//         f: impl FnOnce(ProjectWorld) -> T + Send + 'static,
//     ) -> T {
//         self.typst_thread
//             .run_with_world(self.main_project, self.main, f)
//             .await
//     }
// }


// ==================
// source: github.com/nvarner/typst-lsp

// pub enum WorldBuilder<'a> {
//     MainUri(&'a Url),
//     MainAndProject(Source, Project),
// }

// impl<'a> WorldBuilder<'a> {
//     async fn main_project(self, workspace: &Arc<RwLock<Workspace>>) -> FsResult<(Source, Project)> {
//         match self {
//             Self::MainUri(uri) => {
//                 let workspace = Arc::clone(workspace).read_owned().await;
//                 let full_id = workspace.full_id(uri)?;
//                 let source = workspace.read_source(uri)?;
//                 let project = Project::new(full_id.package(), workspace);
//                 Ok((source, project))
//             }
//             Self::MainAndProject(main, project) => Ok((main, project)),
//         }
//     }
// }

// impl<'a> From<&'a Url> for WorldBuilder<'a> {
//     fn from(uri: &'a Url) -> Self {
//         Self::MainUri(uri)
//     }
// }

// impl From<(Source, Project)> for WorldBuilder<'static> {
//     fn from((main, project): (Source, Project)) -> Self {
//         Self::MainAndProject(main, project)
//     }
// }

// ==================