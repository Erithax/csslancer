use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{reload, Registry};

use std::ffi::OsStr;
use std::fmt::{self, Write};
use std::path::Path;

use tokio::runtime::Handle;
use tower_lsp::lsp_types::MessageType;
use tower_lsp::Client;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Metadata, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;
use console_subscriber;
use tracing_chrome::ChromeLayerBuilder;

use crate::services::CssLancerServer;

#[tracing::instrument]
pub async fn logitgood(msg: &str) {
    tokio::time::sleep(tokio::time::Duration::from_nanos(100)).await;
}

pub fn tracing_init() -> (reload::Handle<Option<LspLayer>, Registry>, tracing_chrome::FlushGuard) {
    let (lsp_layer, lsp_layer_handle) = reload::Layer::new(None);

    let console_layer = console_subscriber::spawn();

    let chrome_trace_file = std::fs::File::create(
        format!("D:/CsslancerTrace__{}.json", chrono::DateTime::naive_local(&chrono::Local::now()).format("%Y-%m-%d__%H-%M-%S"))
    ).expect("could not make csslancer trace file");

    let (chrome_layer, _guard) = ChromeLayerBuilder::new()
        .trace_style(tracing_chrome::TraceStyle::Threaded)
        .include_args(true)
        .writer(chrome_trace_file)
        .build();

    tracing_subscriber::registry()
        .with(lsp_layer)
        .with(console_layer)
        .with(chrome_layer)
        .init();

    (lsp_layer_handle, _guard)
}

pub fn tracing_shutdown() {
}

impl CssLancerServer {
    pub fn tracing_init(&self) {
        let lsp_layer = LspLayer::new(self.client.clone());
        self.lsp_tracing_layer_handle
            .reload(Some(lsp_layer))
            .expect("should be able to replace layer, since it should only fail when there is a larger issue with the `Subscriber`");
    }
}

pub struct LspLayer {
    client: Client,
}

impl LspLayer {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn should_skip(event: &Event) -> bool {
        // these events are emitted when logging to client, causing a recursive chain reaction
        event.metadata().target().contains("codec")
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for LspLayer {
    fn on_event<'b>(&self, event: &Event<'b>, _ctx: Context<S>) {
        if Self::should_skip(event) {
            return;
        }

        if let Ok(handle) = Handle::try_current() {
            let client = self.client.clone();
            let metadata: &Metadata<'b> = event.metadata();

            let message_type: MessageType = level_to_message_type(*metadata.level());

            let line_info: (Option<&'b str>, _) = (metadata.file(), metadata.line());
            let mut message = match line_info {
                (Some(file), Some(line)) => format!("{file}:{line} {{"),
                (Some(file), None) => format!("{file} {{"),
                (None, _) => "{".to_owned(),
            };

            event.record(&mut LspVisit::with_string(&mut message));

            message.push_str(" }");

            handle.spawn(async move {
                client.log_message(message_type, message).await;
            });
        }
    }
}

struct LspVisit<'a> {
    message: &'a mut String,
}

impl<'a> LspVisit<'a> {
    pub fn with_string(string: &'a mut String) -> Self {
        Self { message: string }
    }
}

impl<'a> Visit for LspVisit<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        write!(self.message, " {} = {:?};", field.name(), value).unwrap();
    }
}

fn level_to_message_type(level: Level) -> MessageType {
    match level {
        Level::ERROR => MessageType::ERROR,
        Level::WARN => MessageType::WARNING,
        Level::INFO => MessageType::INFO,
        Level::DEBUG | Level::TRACE => MessageType::LOG,
    }
}
