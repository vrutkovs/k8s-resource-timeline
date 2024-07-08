use chrono::{DateTime, Utc};
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
pub enum ChangeType {
    Metadata,
    Spec,
    Status,
}

pub struct ObjectDiff {
    resource_name: String,
    diff: String,
    change: ChangeType,
    timestamp: DateTime<Utc>,
}

impl std::fmt::Display for ObjectDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: resource {} change {:?} diff\n{}",
            self.timestamp, self.resource_name, self.change, self.diff
        )
    }
}

pub trait ExtraInfo<T> {
    fn resource_name(&self) -> String;
    fn change_type(&self, previous: &T) -> ChangeType;
}

pub trait WatchDiff<T> {
    fn yaml(&self) -> String;
    fn diff(&self, previous: &T) -> ObjectDiff;
}

impl<T> WatchDiff<T> for T
where
    T: kube::Resource + k8s_openapi::serde::Serialize + ExtraInfo<T>,
{
    fn yaml(&self) -> String {
        serde_yaml::to_string(&self).unwrap()
    }
    fn diff(&self, previous: &T) -> ObjectDiff {
        let src: String = previous.yaml();
        let dst: String = self.yaml();
        let input = InternedInput::new(src.as_str(), dst.as_str());
        ObjectDiff {
            timestamp: Utc::now(),
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

impl ExtraInfo<Pod> for Pod {
    fn resource_name(&self) -> String {
        format!(
            "namespace {} pod {}",
            self.namespace().unwrap_or("".into()),
            self.name_any()
        )
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
}

impl ExtraInfo<ConfigMap> for ConfigMap {
    fn resource_name(&self) -> String {
        format!(
            "namespace {} configmap {}",
            self.namespace().unwrap_or("".into()),
            self.name_any()
        )
    }
    fn change_type(&self, previous: &ConfigMap) -> ChangeType {
        if self.data != previous.data {
            return ChangeType::Spec;
        }
        ChangeType::Metadata
    }
}

impl ExtraInfo<Secret> for Secret {
    fn resource_name(&self) -> String {
        format!(
            "namespace {} secret {}",
            self.namespace().unwrap_or("".into()),
            self.name_any()
        )
    }
    fn change_type(&self, previous: &Secret) -> ChangeType {
        if self.data != previous.data {
            return ChangeType::Spec;
        }
        ChangeType::Metadata
    }
}

impl ExtraInfo<Node> for Node {
    fn resource_name(&self) -> String {
        format!("node {}", self.name_any())
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
}

pub async fn watch_resource<T>(
    client: Client,
) -> impl Stream<Item = Result<ObjectDiff, anyhow::Error>>
where
    T: kube::Resource
        + std::clone::Clone
        + std::marker::Send
        + std::fmt::Debug
        + WatchDiff<T>
        + ExtraInfo<T>
        + for<'de> k8s_openapi::serde::Deserialize<'de>,
    <T as kube::Resource>::DynamicType:
        std::clone::Clone + std::default::Default + std::cmp::Eq + std::hash::Hash,
{
    stream! {
        let mut cache: HashMap<String, T> = HashMap::new();

        let client_api: Api<T> = Api::all(client);
        let (_, writer) = reflector::store::<T>();
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
