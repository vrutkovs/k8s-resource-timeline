use crate::watcher;

use k8s_openapi::api::core::v1::{ConfigMap, Node, Pod, Secret};
use kube::Client;

use merge_streams::MergeStreams;

use axum::response::IntoResponse;
use axum_streams::*;

pub async fn watch() -> impl IntoResponse {
    let client = Client::try_default()
        .await
        .expect("client failed to connect");

    let pod_stream = watcher::watch_resource::<Pod>(client.clone()).await;
    let cm_stream = watcher::watch_resource::<ConfigMap>(client.clone()).await;
    let secret_stream = watcher::watch_resource::<Secret>(client.clone()).await;
    let node_stream = watcher::watch_resource::<Node>(client.clone()).await;
    let s = (pod_stream, cm_stream, secret_stream, node_stream).merge();

    StreamBodyAsOptions::new().json_array_with_errors(s)
}
