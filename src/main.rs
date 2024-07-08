use std::collections::HashMap;

use async_stream::stream;
use futures::TryStreamExt;
use futures_core::stream::Stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use merge_streams::MergeStreams;

use k8s_openapi::api::core::v1::{ConfigMap, Node, Pod, Secret};
use kube::{
    api::Api,
    runtime::{predicates, reflector, watcher, WatchStreamExt},
    Client, ResourceExt,
};
use tracing::*;

use imara_diff::intern::InternedInput;
use imara_diff::{diff, Algorithm, UnifiedDiffBuilder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let pod_stream = watch_pods(client.clone()).await;
    let cm_stream = watch_configmaps(client.clone()).await;
    let secret_stream = watch_secrets(client.clone()).await;
    let node_stream = watch_nodes(client.clone()).await;
    let mut s = (pod_stream, cm_stream, secret_stream, node_stream).merge();

    while let Some(maybe_value) = s.next().await {
        if let Ok(value) = maybe_value {
            println!("{}", value);
        }
    }

    Ok(())
}

async fn watch_pods(client: Client) -> impl Stream<Item = Result<String, anyhow::Error>> {
    let mut cache: HashMap<String, String> = HashMap::new();
    stream! {
        let client_api: Api<Pod> = Api::all(client);
        let (_, writer) = reflector::store::<Pod>();
        let stream = watcher(client_api, watcher::Config::default().any_semantic())
            .default_backoff()
            .reflect(writer)
            .applied_objects()
            .predicate_filter(predicates::resource_version);
        pin_mut!(stream);

        while let Some(pod) = stream.try_next().await? {
            let resource_name = format!(
                "namespace {} pod {}",
                pod.namespace().unwrap_or("".into()),
                pod.name_any()
            );
            let previous_yaml: String = cache.get(&resource_name).unwrap_or(&"".into()).into();
            let yaml = serde_yaml::to_string(&pod).unwrap();
            let input = InternedInput::new(previous_yaml.as_str(), yaml.as_str());
            let diff = diff(
                Algorithm::Histogram,
                &input,
                UnifiedDiffBuilder::new(&input),
            );
            cache.entry(resource_name).or_insert(yaml);
            yield Ok(diff);
        }
    }
}

async fn watch_configmaps(client: Client) -> impl Stream<Item = Result<String, anyhow::Error>> {
    let mut cache: HashMap<String, String> = HashMap::new();
    stream! {
        let client_api: Api<ConfigMap> = Api::all(client);
        let (_, writer) = reflector::store::<ConfigMap>();
        let stream = watcher(client_api, watcher::Config::default().any_semantic())
            .default_backoff()
            .reflect(writer)
            .applied_objects()
            .predicate_filter(predicates::resource_version);
        pin_mut!(stream);

        while let Some(configmap) = stream.try_next().await? {
            let resource_name = format!(
                "namespace {} configmap {}",
                configmap.namespace().unwrap_or("".into()),
                configmap.name_any()
            );
            let previous_yaml: String = cache.get(&resource_name).unwrap_or(&"".into()).into();
            let yaml = serde_yaml::to_string(&configmap).unwrap();
            let input = InternedInput::new(previous_yaml.as_str(), yaml.as_str());
            let diff = diff(
                Algorithm::Histogram,
                &input,
                UnifiedDiffBuilder::new(&input),
            );
            cache.entry(resource_name).or_insert(yaml);
            yield Ok(diff);
        }
    }
}

async fn watch_secrets(client: Client) -> impl Stream<Item = Result<String, anyhow::Error>> {
    let mut cache: HashMap<String, String> = HashMap::new();
    stream! {
        let client_api: Api<Secret> = Api::all(client);
        let (_, writer) = reflector::store::<Secret>();
        let stream = watcher(client_api, watcher::Config::default().any_semantic())
            .default_backoff()
            .reflect(writer)
            .applied_objects()
            .predicate_filter(predicates::resource_version);
        pin_mut!(stream);

        while let Some(secret) = stream.try_next().await? {
            let resource_name = format!(
                "namespace {} secret {}",
                secret.namespace().unwrap_or("".into()),
                secret.name_any()
            );
            let previous_yaml: String = cache.get(&resource_name).unwrap_or(&"".into()).into();
            let yaml = serde_yaml::to_string(&secret).unwrap();
            let input = InternedInput::new(previous_yaml.as_str(), yaml.as_str());
            let diff = diff(
                Algorithm::Histogram,
                &input,
                UnifiedDiffBuilder::new(&input),
            );
            cache.entry(resource_name).or_insert(yaml);
            yield Ok(diff);
        }
    }
}

async fn watch_nodes(client: Client) -> impl Stream<Item = Result<String, anyhow::Error>> {
    let mut cache: HashMap<String, String> = HashMap::new();
    stream! {
        let client_api: Api<Node> = Api::all(client);
        let (_, writer) = reflector::store::<Node>();
        let stream = watcher(client_api, watcher::Config::default().any_semantic())
            .default_backoff()
            .reflect(writer)
            .applied_objects()
            .predicate_filter(predicates::resource_version);
        pin_mut!(stream);

        while let Some(node) = stream.try_next().await? {
            let resource_name = format!(
                "node {}",
                node.name_any()
            );
            let previous_yaml: String = cache.get(&resource_name).unwrap_or(&"".into()).into();
            let yaml = serde_yaml::to_string(&node).unwrap();
            let input = InternedInput::new(previous_yaml.as_str(), yaml.as_str());
            let diff = diff(
                Algorithm::Histogram,
                &input,
                UnifiedDiffBuilder::new(&input),
            );
            cache.entry(resource_name).or_insert(yaml);
            yield Ok(diff);
        }
    }
}
