use crate::stream::watch_with_timeout;
use futures_util::stream;
use futures_util::{Stream, StreamExt};
use std::convert::Infallible;
use stream_throttle::{ThrottlePool, ThrottleRate, ThrottledStream};

use askama::Template;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use tokio::time::Duration;

use axum::Json;
use serde::{de, Deserialize, Deserializer};
use std::{fmt, str::FromStr};

use serde_json::json;

#[derive(Template)]
#[template(path = "main.html", escape = "none")]
struct TimelineTemplate {}

pub async fn timeline() -> impl IntoResponse {
    // let diffs = watch_with_timeout(params.timeout.unwrap_or(60)).await;
    // let intervals = serde_json::to_string(&diffs).expect("serizalize to json works");
    let template = TimelineTemplate {};
    let reply_html = template.render().unwrap();
    (StatusCode::OK, Html(reply_html).into_response())
}

pub async fn events() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let output = json!({"locator":{"type":"Pod","keys":{"namespace":"openshift-marketplace","name":"redhat-operators-g7xxx","uid":"a69f0887-05c0-43fa-b02f-1f5baca954d1"}},"change":{"type":"Status","diff":"@@ -215,8 +215,6 @@\n             .: {}\n             f:lastProbeTime: {}\n             f:lastTransitionTime: {}\n-            f:message: {}\n-            f:reason: {}\n             f:status: {}\n             f:type: {}\n           k:{\"type\":\"Initialized\"}:\n@@ -229,8 +227,6 @@\n             .: {}\n             f:lastProbeTime: {}\n             f:lastTransitionTime: {}\n-            f:message: {}\n-            f:reason: {}\n             f:status: {}\n             f:type: {}\n         f:containerStatuses: {}\n@@ -257,7 +253,7 @@\n     kind: CatalogSource\n     name: redhat-operators\n     uid: 9656305a-48b9-4cc6-94a2-7c374be92414\n-  resourceVersion: '26415'\n+  resourceVersion: '26417'\n   uid: a69f0887-05c0-43fa-b02f-1f5baca954d1\n spec:\n   containers:\n@@ -449,15 +445,11 @@\n   - lastTransitionTime: 2024-07-19T14:13:23Z\n     status: 'True'\n     type: Initialized\n-  - lastTransitionTime: 2024-07-19T14:13:20Z\n-    message: 'containers with unready status: [registry-server]'\n-    reason: ContainersNotReady\n-    status: 'False'\n+  - lastTransitionTime: 2024-07-19T14:13:31Z\n+    status: 'True'\n     type: Ready\n-  - lastTransitionTime: 2024-07-19T14:13:20Z\n-    message: 'containers with unready status: [registry-server]'\n-    reason: ContainersNotReady\n-    status: 'False'\n+  - lastTransitionTime: 2024-07-19T14:13:31Z\n+    status: 'True'\n     type: ContainersReady\n   - lastTransitionTime: 2024-07-19T14:13:20Z\n     status: 'True'\n@@ -468,7 +460,7 @@\n     imageID: quay.io/openshift-release-dev/ocp-v4.0-art-dev@sha256:6c3f49444d418a33128125a509a3ba91f6afe35e6c1385dd5738d10208556a58\n     lastState: {}\n     name: registry-server\n-    ready: false\n+    ready: true\n     restartCount: 0\n     started: true\n     state:\n"},"from":"2024-07-19T14:13:34.813733850Z","to":"2024-07-19T14:13:34.813738416Z"}
    );
    let rate = ThrottleRate::new(1, Duration::new(1, 0));
    let pool = ThrottlePool::new(rate);
    let stream = stream::repeat_with(move || Event::default().data(output.to_string()))
        .map(Ok)
        .throttle(pool);

    Sse::new(stream).keep_alive(KeepAlive::default())
}
