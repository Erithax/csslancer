pub mod css_selection_range;
pub mod css_validation;
pub mod semantic_tokens;
pub mod hover;
pub mod selector_printing;

use crate::css_language_types::HoverSettings;
use crate::data::data_manager::CssDataManager;
use crate::logging::LspLayer;
use crate::workspace::source::Source;
use crate::{
    config::{Config, SemanticTokensMode},
    ext::InitializeParamsExt,
    services::semantic_tokens::{
        get_semantic_tokens_registration, get_semantic_tokens_unregistration,
    },
    workspace::FsError,
};
use anyhow::Context;
use futures::FutureExt;
use lsp_types::*;
use lsp_types::{
    CompletionParams, CompletionResponse, DiagnosticOptions, DiagnosticServerCapabilities,
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    DocumentSymbolResponse, ExecuteCommandParams, Hover, HoverParams, InitializeResult,
    ServerCapabilities, SignatureHelp, SignatureHelpParams, WorkDoneProgressOptions,
};
use serde_json::Value as JsonValue;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use tower_lsp::{
    jsonrpc::{self, Error},
    lsp_types::{InitializeParams, Url},
    Client, LanguageServer,
};
use tracing::{error, trace, warn};
use tracing_subscriber::{reload, Registry};

use crate::config::ConstConfig;
use crate::workspace::Workspace;

use std::sync::{Arc, RwLock as SyncRwLock};

use self::semantic_tokens::{get_semantic_tokens_options, SemanticTokenCache};

pub struct CssLancerServer {
    pub client: Client,
    workspace: OnceLock<RwLock<Workspace>>,
    const_config: OnceLock<ConstConfig>,
    client_capabilities: OnceLock<ClientCapabilities>,
    client_supports_markdown: OnceLock<bool>,
    css_data_manager: CssDataManager, // = SelectorPrinting too
    config: Arc<RwLock<Config>>,
    semantic_tokens_delta_cache: Arc<SyncRwLock<SemanticTokenCache>>,
    pub lsp_tracing_layer_handle: reload::Handle<Option<LspLayer>, Registry>,
}

impl CssLancerServer {
    pub fn new(
        client: Client,
        lsp_tracing_layer_handle: reload::Handle<Option<LspLayer>, Registry>,
    ) -> Self {
        Self {
            workspace: Default::default(),
            const_config: Default::default(),
            client_capabilities: Default::default(),
            client_supports_markdown: Default::default(),
            css_data_manager: CssDataManager::new(true, None),
            config: Default::default(),
            client,
            semantic_tokens_delta_cache: Arc::new(SyncRwLock::new(SemanticTokenCache::default())),
            lsp_tracing_layer_handle,
        }
    }

    pub fn new_dud() -> Self {
        //let (lsp_tracing_layer_handle, _chrome_trace_guard) = crate::logging::tracing_init();

        let (_, lsp_tracing_layer_handle) = reload::Layer::new(None);

        let lsp_tracing_layer_handle_clone = lsp_tracing_layer_handle.clone();

        let (service, _) =
            tower_lsp::LspService::new(move |client| CssLancerServer::new(client, lsp_tracing_layer_handle_clone));
    
        let client = service.inner().client.clone();

        //crate::logging::tracing_shutdown();

        Self::new(client, lsp_tracing_layer_handle)
    }

    pub async fn workspace_read(&self) -> tokio::sync::RwLockReadGuard<'_, Workspace> {
        self.workspace
            .get()
            .expect("workspace should be initialized")
            .read()
            .await
    }

    pub async fn workspace_write(&self) -> tokio::sync::RwLockWriteGuard<'_, Workspace> {
        self.workspace
            .get()
            .expect("workspace should be initialized")
            .write()
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn source_read(
        &self,
        url: &Url,
    ) -> Result<tokio::sync::RwLockReadGuard<'_, Source>, FsError> {
        // TODO: do this without double HashMap access (get_document_ref(url))
        let w = self.workspace_read().await;

        {
            w.get_document_ref(url)?;
        }

        return Ok(tokio::sync::RwLockReadGuard::map(w, |w| {
            w.get_document_ref(url).unwrap()
        }));
    }

    pub fn const_config(&self) -> &ConstConfig {
        return self
            .const_config
            .get()
            .expect("const config should be initialized");
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

        self.client_capabilities
            .set(params.capabilities.clone())
            .expect("client capabilities should not yet be initialized");


        if let Some(init) = &params.initialization_options {
            warn!("found init options");
            let mut config = self.config.write().await;
            config
                .update(init)
                .await
                .as_ref()
                .map_err(ToString::to_string)
                .map_err(jsonrpc::Error::invalid_params)?;
        }

        if let Err(err) = self.workspace_write().await.register_files() {
            tracing::error!(%err, "could not register workspace files on init");
            return Err(jsonrpc::Error::internal_error());
        }

        let semantic_tokens_provider =
            if self.config.read().await.semantic_tokens == SemanticTokensMode::Enable {
                if !params.supports_semantic_tokens_dynamic_registration() {
                    Some(get_semantic_tokens_options().into())
                } else {
                    None
                }
            } else {
                None
            };

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
                            work_done_progress: None,
                        },
                    },
                )),
                semantic_tokens_provider,
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    #[tracing::instrument(skip_all)]
    async fn initialized(&self, _: InitializedParams) {
        tracing::trace!("INITIALIZED");

        let const_config = self.const_config();
        let mut config = self.config.write().await;

        if const_config.supports_semantic_tokens_dynamic_registration {
            tracing::trace!("setting up to dynamically register semantic token support");

            let client = self.client.clone();
            let register = move || {
                trace!("dynamically registering semantic tokens");
                let client = client.clone();
                async move {
                    let options = get_semantic_tokens_options();
                    client
                        .register_capability(vec![get_semantic_tokens_registration(options)])
                        .await
                        .context("could not register semantic tokens")
                }
            };

            let client = self.client.clone();
            let unregister = move || {
                trace!("unregistering semantic tokens");
                let client = client.clone();
                async move {
                    client
                        .unregister_capability(vec![get_semantic_tokens_unregistration()])
                        .await
                        .context("could not unregister semantic tokens")
                }
            };

            if config.semantic_tokens == SemanticTokensMode::Enable {
                if let Some(err) = register().await.err() {
                    error!(%err, "could not dynamically register semantic tokens");
                }
            }

            config.listen_semantic_tokens(Box::new(move |mode| match mode {
                SemanticTokensMode::Enable => register().boxed(),
                SemanticTokensMode::Disable => unregister().boxed(),
            }));
        }
        trace!("end of initialized");
    }

    #[tracing::instrument(skip_all)]
    async fn shutdown(&self) -> jsonrpc::Result<()> {
        tracing::trace!("SHUTDOWN");
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        trace!("did_open()");
        let doc = params.text_document;
        let url = doc.uri;
        let src = Source::new(url.clone(), &doc.text, doc.version);

        let mut workspace = self.workspace_write().await;
        workspace.open(url.clone(), src);
        drop(workspace);

        if let Err(err) = self.on_source_changed(&url).await {
            tracing::error!(%err, %url, "could not handle source change");
        };
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        trace!("did_close()");
        let uri = params.text_document.uri;

        let mut workspace = self.workspace_write().await;
        workspace.close(&uri);
        drop(workspace);

        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        trace!("did_change()");
        self.client
            .log_message(MessageType::WARNING, "did_change()")
            .await;
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
        trace!("did_save()");
        let uri = params.text_document.uri;

        if let Err(err) = self.on_source_changed(&uri).await {
            tracing::error!(%err, %uri, "could not handle source save");
        }
    }

    async fn did_change_watched_files(&self, _params: DidChangeWatchedFilesParams) {}

    async fn did_change_workspace_folders(&self, _params: DidChangeWorkspaceFoldersParams) {}

    async fn execute_command(
        &self,
        _params: ExecuteCommandParams,
    ) -> jsonrpc::Result<Option<JsonValue>> {
        Err(Error::invalid_request())
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document_position_params.text_document.uri))]
    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        trace!("hover()");
        let url = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        match self.source_read(&url).await {
            Err(err) => {
                tracing::error!(%err, %url, "could not handle hover (could not lock source file)");
                return jsonrpc::Result::Err(jsonrpc::Error::internal_error());
            }
            Ok(o) => return 
                self.get_hover(&o, position, &Some(HoverSettings{documentation: true, references: true})).map_err(|err| {
                    error!(%err, %url, "error getting hover");
                    jsonrpc::Error::internal_error()
                })
            ,
        }
    }

    async fn completion(
        &self,
        _params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        Ok(None)
    }

    async fn signature_help(
        &self,
        _params: SignatureHelpParams,
    ) -> jsonrpc::Result<Option<SignatureHelp>> {
        Ok(None)
    }

    async fn document_symbol(
        &self,
        _params: DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<DocumentSymbolResponse>> {
        Ok(None)
    }

    async fn symbol(
        &self,
        _params: WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<Vec<SymbolInformation>>> {
        Ok(None)
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        trace!("semantic_tokens_full()");
        let uri = params.text_document.uri;

        let Ok(src_read_guard) = self.source_read(&uri).await else {
            return Err(tower_lsp::jsonrpc::Error::invalid_request());
        };
        let (tokens, result_id) = self.get_semantic_tokens_full(&src_read_guard);
        Ok(Some(
            SemanticTokens {
                result_id: Some(result_id),
                data: tokens,
            }
            .into(),
        ))
    }

    #[tracing::instrument(skip_all, fields(uri = %params.text_document.uri))]
    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> jsonrpc::Result<Option<SemanticTokensFullDeltaResult>> {
        warn!("semantic_tokens_full_delta()");
        let uri = params.text_document.uri;
        let previous_result_id = params.previous_result_id;

        let Ok(src_read_guard) = self.source_read(&uri).await else {
            return Err(tower_lsp::jsonrpc::Error::invalid_request());
        };

        let (tokens, result_id) =
            self.try_semantic_tokens_delta_from_result_id(&src_read_guard, &previous_result_id);

        match tokens {
            Ok(edits) => Ok(Some(
                SemanticTokensDelta {
                    result_id: Some(result_id),
                    edits,
                }
                .into(),
            )),
            Err(tokens) => Ok(Some(
                SemanticTokens {
                    result_id: Some(result_id),
                    data: tokens,
                }
                .into(),
            )),
        }
    }

    #[tracing::instrument(skip(self))]
    async fn did_change_configuration(&self, _params: DidChangeConfigurationParams) {}

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> jsonrpc::Result<Option<Vec<SelectionRange>>> {
        warn!("selection_range()");
        let url = params.text_document.uri;
        match self.source_read(&url).await {
            Err(err) => {
                tracing::error!(%err, %url, "could not handle source change");
                return jsonrpc::Result::Err(jsonrpc::Error::internal_error());
            }
            Ok(o) => return Ok(Some(self.get_selection_ranges(&o, &params.positions))),
        }
    }

    async fn formatting(
        &self,
        _params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        Ok(None)
    }
}
