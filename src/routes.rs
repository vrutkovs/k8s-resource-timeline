use futures_util::{Stream, StreamExt};
use std::convert::Infallible;

use askama::Template;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};

use crate::watcher;

use k8s_openapi::api::core::v1::{ConfigMap, Node, Pod, Secret};
use kube::Client;

use merge_streams::MergeStreams;

#[derive(Template)]
#[template(path = "main.html", escape = "none")]
struct TimelineTemplate {}

pub async fn timeline() -> impl IntoResponse {
    // TODO: doesn't template anything, needs to be static
    let template = TimelineTemplate {};
    let reply_html = template.render().unwrap();
    (StatusCode::OK, Html(reply_html).into_response())
}

pub async fn events() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let client = Client::try_default()
        .await
        .expect("client failed to connect");

    let pod_stream = watcher::watch_resource::<Pod>(client.clone()).await;
    let cm_stream = watcher::watch_resource::<ConfigMap>(client.clone()).await;
    let secret_stream = watcher::watch_resource::<Secret>(client.clone()).await;
    let node_stream = watcher::watch_resource::<Node>(client.clone()).await;
    let s = (pod_stream, cm_stream, secret_stream, node_stream).merge();

    let sse_events = s
        .map(|e| {
            Event::default()
                .data(serde_json::to_string(&e.expect("result")).expect("can serialize"))
        })
        .map(Ok);

    Sse::new(sse_events).keep_alive(KeepAlive::default())
}
