use crate::watcher::{self, ObjectDiff};
use tokio::select;
use tokio::time::Duration;

use k8s_openapi::api::core::v1::{ConfigMap, Node, Pod, Secret};
use kube::Client;

use futures::StreamExt;
use merge_streams::MergeStreams;

pub async fn watch_with_timeout(timeout: u64) -> Vec<ObjectDiff> {
    let client = Client::try_default()
        .await
        .expect("client failed to connect");

    let pod_stream = watcher::watch_resource::<Pod>(client.clone()).await;
    let cm_stream = watcher::watch_resource::<ConfigMap>(client.clone()).await;
    let secret_stream = watcher::watch_resource::<Secret>(client.clone()).await;
    let node_stream = watcher::watch_resource::<Node>(client.clone()).await;
    let mut s = (pod_stream, cm_stream, secret_stream, node_stream).merge();

    let mut output: Vec<ObjectDiff> = vec![];
    let iteration = async {
        while let Some(n) = s.next().await {
            output.push(n.unwrap());
        }
        output.clone()
    };
    select! {
        result = iteration => result,
        _ = tokio::time::sleep(Duration::from_secs(timeout)) => output.clone()
    }
}
