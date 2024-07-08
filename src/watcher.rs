use std::collections::HashMap;

use async_stream::stream;
use futures::TryStreamExt;
use futures_core::stream::Stream;
use futures_util::pin_mut;

use k8s_openapi::api::core::v1::{ConfigMap, Node, Pod, Secret};
use kube::{
    api::Api,
    runtime::{predicates, reflector, watcher, WatchStreamExt},
    Client, ResourceExt,
};

use imara_diff::intern::InternedInput;
use imara_diff::{diff, Algorithm, UnifiedDiffBuilder};

#[derive(Debug)]
enum ChangeType {
    Metadata,
    Spec,
    Status,
}

pub struct ObjectDiff {
    resource_name: String,
    diff: String,
    change: ChangeType,
}

impl std::fmt::Display for ObjectDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: change {:?} diff\n{}",
            self.resource_name, self.change, self.diff
        )
    }
}

trait WatchDiff<T> {
    fn resource_name(&self) -> String;
    fn yaml(&self) -> String;
    fn change_type(&self, previous: &T) -> ChangeType;
    fn diff(&self, previous: &T) -> ObjectDiff;
}

impl WatchDiff<Pod> for Pod {
    fn resource_name(&self) -> String {
        format!(
            "namespace {} pod {}",
            self.namespace().unwrap_or("".into()),
            self.name_any()
        )
    }
    fn yaml(&self) -> String {
        serde_yaml::to_string(&self).unwrap()
    }
    fn change_type(&self, previous: &Pod) -> ChangeType {
        if self.spec != previous.spec {
            return ChangeType::Spec;
        }
        if self.status != previous.status {
            return ChangeType::Status;
        }
        ChangeType::Metadata
    }
    fn diff(&self, previous: &Pod) -> ObjectDiff {
        let src: String = previous.yaml();
        let dst: String = self.yaml();
        let input = InternedInput::new(src.as_str(), dst.as_str());
        ObjectDiff {
            change: self.change_type(previous),
            resource_name: self.resource_name(),
            diff: diff(
                Algorithm::Histogram,
                &input,
                UnifiedDiffBuilder::new(&input),
            ),
        }
    }
}

impl WatchDiff<ConfigMap> for ConfigMap {
    fn resource_name(&self) -> String {
        format!(
            "namespace {} configmap {}",
            self.namespace().unwrap_or("".into()),
            self.name_any()
        )
    }
    fn yaml(&self) -> String {
        serde_yaml::to_string(&self).unwrap()
    }
    fn change_type(&self, previous: &ConfigMap) -> ChangeType {
        if self.data != previous.data {
            return ChangeType::Spec;
        }
        ChangeType::Metadata
    }
    fn diff(&self, previous: &ConfigMap) -> ObjectDiff {
        let src: String = previous.yaml();
        let dst: String = self.yaml();
        let input = InternedInput::new(src.as_str(), dst.as_str());
        ObjectDiff {
            change: self.change_type(previous),
            resource_name: self.resource_name(),
            diff: diff(
                Algorithm::Histogram,
                &input,
                UnifiedDiffBuilder::new(&input),
            ),
        }
    }
}

impl WatchDiff<Secret> for Secret {
    fn resource_name(&self) -> String {
        format!(
            "namespace {} secret {}",
            self.namespace().unwrap_or("".into()),
            self.name_any()
        )
    }
    fn yaml(&self) -> String {
        serde_yaml::to_string(&self).unwrap()
    }
    fn change_type(&self, previous: &Secret) -> ChangeType {
        if self.data != previous.data {
            return ChangeType::Spec;
        }
        ChangeType::Metadata
    }
    fn diff(&self, previous: &Secret) -> ObjectDiff {
        let src: String = previous.yaml();
        let dst: String = self.yaml();
        let input = InternedInput::new(src.as_str(), dst.as_str());
        ObjectDiff {
            change: self.change_type(previous),
            resource_name: self.resource_name(),
            diff: diff(
                Algorithm::Histogram,
                &input,
                UnifiedDiffBuilder::new(&input),
            ),
        }
    }
}

impl WatchDiff<Node> for Node {
    fn resource_name(&self) -> String {
        format!("node {}", self.name_any())
    }
    fn yaml(&self) -> String {
        serde_yaml::to_string(&self).unwrap()
    }
    fn change_type(&self, previous: &Node) -> ChangeType {
        if self.spec != previous.spec {
            return ChangeType::Spec;
        }
        if self.status != previous.status {
            return ChangeType::Status;
        }
        ChangeType::Metadata
    }
    fn diff(&self, previous: &Node) -> ObjectDiff {
        let src: String = previous.yaml();
        let dst: String = self.yaml();
        let input = InternedInput::new(src.as_str(), dst.as_str());
        ObjectDiff {
            change: self.change_type(previous),
            resource_name: self.resource_name(),
            diff: diff(
                Algorithm::Histogram,
                &input,
                UnifiedDiffBuilder::new(&input),
            ),
        }
    }
}

pub async fn watch_pods(client: Client) -> impl Stream<Item = Result<ObjectDiff, anyhow::Error>> {
    stream! {
        let mut cache: HashMap<String, Pod> = HashMap::new();

        let client_api: Api<Pod> = Api::all(client);
        let (_, writer) = reflector::store::<Pod>();
        let stream = watcher(client_api, watcher::Config::default().any_semantic())
            .default_backoff()
            .reflect(writer)
            .applied_objects()
            .predicate_filter(predicates::resource_version);
        pin_mut!(stream);

        while let Some(current) = stream.try_next().await? {
            let mut diff : Option<ObjectDiff> = None;
            cache.entry(current.resource_name()).and_modify(|previous| {
                diff = Some(current.diff(previous));
                *previous = current.clone();
            }).or_insert(current);
            if let Some(diff_value) = diff {
                yield Ok(diff_value)
            }
        }
    }
}

pub async fn watch_configmaps(
    client: Client,
) -> impl Stream<Item = Result<ObjectDiff, anyhow::Error>> {
    let mut cache: HashMap<String, ConfigMap> = HashMap::new();
    stream! {
        let client_api: Api<ConfigMap> = Api::all(client);
        let (_, writer) = reflector::store::<ConfigMap>();
        let stream = watcher(client_api, watcher::Config::default().any_semantic())
            .default_backoff()
            .reflect(writer)
            .applied_objects()
            .predicate_filter(predicates::resource_version);
        pin_mut!(stream);

        while let Some(current) = stream.try_next().await? {
            let mut diff : Option<ObjectDiff> = None;
            cache.entry(current.resource_name()).and_modify(|previous| {
                diff = Some(current.diff(previous));
                *previous = current.clone();
            }).or_insert(current);
            if let Some(diff_value) = diff {
                yield Ok(diff_value)
            }
        }
    }
}

pub async fn watch_secrets(
    client: Client,
) -> impl Stream<Item = Result<ObjectDiff, anyhow::Error>> {
    let mut cache: HashMap<String, Secret> = HashMap::new();
    stream! {
        let client_api: Api<Secret> = Api::all(client);
        let (_, writer) = reflector::store::<Secret>();
        let stream = watcher(client_api, watcher::Config::default().any_semantic())
            .default_backoff()
            .reflect(writer)
            .applied_objects()
            .predicate_filter(predicates::resource_version);
        pin_mut!(stream);

        while let Some(current) = stream.try_next().await? {
            let mut diff : Option<ObjectDiff> = None;
            cache.entry(current.resource_name()).and_modify(|previous| {
                diff = Some(current.diff(previous));
                *previous = current.clone();
            }).or_insert(current);
            if let Some(diff_value) = diff {
                yield Ok(diff_value)
            }
        }
    }
}

pub async fn watch_nodes(client: Client) -> impl Stream<Item = Result<ObjectDiff, anyhow::Error>> {
    let mut cache: HashMap<String, Node> = HashMap::new();
    stream! {
        let client_api: Api<Node> = Api::all(client);
        let (_, writer) = reflector::store::<Node>();
        let stream = watcher(client_api, watcher::Config::default().any_semantic())
            .default_backoff()
            .reflect(writer)
            .applied_objects()
            .predicate_filter(predicates::resource_version);
        pin_mut!(stream);

        while let Some(current) = stream.try_next().await? {
            let mut diff : Option<ObjectDiff> = None;
            cache.entry(current.resource_name()).and_modify(|previous| {
                diff = Some(current.diff(previous));
                *previous = current.clone();
            }).or_insert(current);
            if let Some(diff_value) = diff {
                yield Ok(diff_value)
            }
        }
    }
}
