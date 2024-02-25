
use tower_lsp::{LspService, Server};
use csslancer::services::CssLancerServer;
use tracing_subscriber;
use csslancer::logging::{tracing_init, tracing_shutdown, LspLayer};
use tracing_subscriber::{reload, Registry};

#[tokio::main]
async fn main() {
    let (lsp_tracing_layer_handle, _chrome_trace_guard) = tracing_init();
    run(lsp_tracing_layer_handle).await;
    tracing_shutdown();
}

#[tracing::instrument(skip_all)]
async fn run(lsp_tracing_layer_handle: reload::Handle<Option<LspLayer>, Registry>) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(move |client| CssLancerServer::new(client, lsp_tracing_layer_handle));

    Server::new(stdin, stdout, socket).serve(service).await;
}

// fn main() -> Result<()> {

//     let mut builder = Builder::with_level("INFO");
//     let now: DateTime<Local> = Local::now();

//     let file = Some(
//         format!(
//             "C:/users/admin/desktop/csslancer-{}.log",
//             now.format("%Y-%m-%d_%H-%M-%S-%f")
//         )
//     );

//     if let Some(file) = &file {
//         let log_file = std::fs::File::options()
//             .create(true)
//             .append(true)
//             .open(file)
//             .unwrap();

//         builder = builder.with_target_writer("*", new_writer(log_file));
//     } else {
//         builder = builder.with_target_writer("*", new_writer(std::io::stderr()))
//     }

//     builder.init();
//     trace!("log options: {:?}", ("INFO", file));

//     start_lsp()?;

//     Ok(())
// }
