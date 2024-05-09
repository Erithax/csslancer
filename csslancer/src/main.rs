use csslancer::logging::{tracing_init, tracing_shutdown};
use csslancer::services::CssLancerServer;
use tower_lsp::{LspService, Server};

// problems rowan/ungrammar
// enum of tokens functions different from enum of nodes?

#[tokio::main]
async fn main() {
    let (lsp_tracing_layer_handle, _chrome_trace_guard) = tracing_init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) =
        LspService::new(move |client| CssLancerServer::new(client, lsp_tracing_layer_handle));
    Server::new(stdin, stdout, socket).serve(service).await;

    tracing_shutdown();
}
