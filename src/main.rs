use futures_util::stream::StreamExt;
use merge_streams::MergeStreams;

use kube::Client;
use tracing::*;

mod watcher;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let pod_stream = watcher::watch_pods(client.clone()).await;
    let cm_stream = watcher::watch_configmaps(client.clone()).await;
    let secret_stream = watcher::watch_secrets(client.clone()).await;
    let node_stream = watcher::watch_nodes(client.clone()).await;
    let mut s = (pod_stream, cm_stream, secret_stream, node_stream).merge();

    while let Some(maybe_value) = s.next().await {
        if let Ok(value) = maybe_value {
            println!("{}", value);
        }
    }

    Ok(())
}
