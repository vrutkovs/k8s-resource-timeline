use std::{collections::HashMap, pin::pin};

use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::Api,
    runtime::{predicates, reflector, watcher, WatchStreamExt},
    Client, ResourceExt,
};
use tracing::*;

use imara_diff::intern::InternedInput;
use imara_diff::{diff, Algorithm, UnifiedDiffBuilder};

struct Cache {
    pub table: HashMap<String, String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let mut pod_table: HashMap<String, String> = HashMap::new();

    let pod_api: Api<Pod> = Api::all(client);
    let (_, pod_writer) = reflector::store::<Pod>();
    let pod_stream = watcher(pod_api, watcher::Config::default().any_semantic())
        .default_backoff()
        .reflect(pod_writer)
        .applied_objects()
        .predicate_filter(predicates::resource_version);
    let mut pod_stream = pin!(pod_stream);

    while let Some(pod) = pod_stream.try_next().await? {
        let resource_name = format!(
            "namespace {} pod {}",
            pod.namespace().unwrap_or("".into()),
            pod.name_any()
        );
        info!(
            "{}: rv {}",
            resource_name,
            pod.resource_version().unwrap_or("".into())
        );

        let previous_yaml: String = pod_table.get(&resource_name).unwrap_or(&"".into()).into();
        let yaml = serde_yaml::to_string(&pod).unwrap();
        let input = InternedInput::new(previous_yaml.as_str(), yaml.as_str());
        let diff = diff(
            Algorithm::Histogram,
            &input,
            UnifiedDiffBuilder::new(&input),
        );
        println!("{}", diff);
        pod_table.entry(resource_name).or_insert(yaml);
    }
    Ok(())
}
