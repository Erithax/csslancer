#![allow(clippy::needless_return)]
#![allow(clippy::new_without_default)]

mod config;
mod css_language_service;
pub mod css_language_types;
mod ext;
pub mod data;
mod interop;
pub mod logging;
pub mod services;
pub mod workspace;
pub mod tokenizer;
pub mod row_parser;

use anyhow::Result;
use log::{error, info, warn};
use lsp_types::{
    InitializeParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    WorkDoneProgressOptions,
};

use lsp_server::Connection;

fn main_loop(connection: Connection, params: serde_json::Value) -> Result<()> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();

    info!("STARTING EXAMPLE MAIN LOOP");

    for msg in &connection.receiver {
        error!("connection received message: {:?}", msg);
    }

    Ok(())
}

pub fn start_lsp() -> Result<()> {
    log::trace!("before");

    // Note that  we must have our logging only write out to stderr.
    log::trace!("bonk");
    info!("starting generic LSP server");

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(lsp_types::CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec!["-".to_string(), "\"".to_string(), " ".to_string()]),
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
            all_commit_characters: None,
            completion_item: None,
        }),

        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),

        ..Default::default()
    })
    .unwrap();

    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params)?;
    io_threads.join()?;

    // Shut down gracefully.
    warn!("shutting down server");
    Ok(())
}
