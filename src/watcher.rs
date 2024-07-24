use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChangeType {
    Metadata,
    Spec,
    Status,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LocatorType {
    Pod,
    Secret,
    ConfigMap,
    Node,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Locator {
    #[serde(rename = "type")]
    _type: LocatorType,
    keys: LocatorKeys,
}

impl std::fmt::Display for Locator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "resource {:?} name {} namespace {:?}",
            self._type, self.keys.name, self.keys.namespace
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Change {
    #[serde(rename = "type")]
    _type: ChangeType,
    diff: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LocatorKeys {
    namespace: Option<String>,
    name: String,
    uid: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectDiff {
    locator: Locator,
    change: Change,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
}

impl std::fmt::Display for ObjectDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: resource {} type {:?} diff\n{}",
            self.from, self.locator, self.change._type, self.change.diff
        )
    }
}

pub trait WatchDiff<T> {
    fn yaml(&self) -> String;
    fn diff(&self, previous: &T) -> ObjectDiff;
    fn locator(&self) -> Locator;
}

impl<T> WatchDiff<T> for T
where
    T: kube::Resource + k8s_openapi::serde::Serialize + ExtraInfo<T>,
{
    fn locator(&self) -> Locator {
        Locator {
            _type: LocatorType::Pod,
            keys: LocatorKeys {
                namespace: self.namespace(),
                name: self.name_any(),
                uid: self.uid().expect("resources have uid"),
            },
        }
    }

    fn yaml(&self) -> String {
        serde_yaml::to_string(&self).unwrap()
    }

    fn diff(&self, previous: &T) -> ObjectDiff {
        let src: String = previous.yaml();
        let dst: String = self.yaml();
        let input = InternedInput::new(src.as_str(), dst.as_str());
        ObjectDiff {
            from: Utc::now(), // TODO: uuugh
            to: Utc::now(),
            locator: self.locator(),
            change: Change {
                _type: self.change_type(previous),
                diff: diff(
                    Algorithm::Histogram,
                    &input,
                    UnifiedDiffBuilder::new(&input),
                ),
            },
        }
    }
}

pub trait ExtraInfo<T> {
    // TODO: make another trait for Resource to get locator_keys
    fn change_type(&self, previous: &T) -> ChangeType;
}

impl ExtraInfo<Pod> for Pod {
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
    fn change_type(&self, previous: &ConfigMap) -> ChangeType {
        if self.data != previous.data {
            return ChangeType::Spec;
        }
        ChangeType::Metadata
    }
}

impl ExtraInfo<Secret> for Secret {
    fn change_type(&self, previous: &Secret) -> ChangeType {
        if self.data != previous.data {
            return ChangeType::Spec;
        }
        ChangeType::Metadata
    }
}

impl ExtraInfo<Node> for Node {
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

#[derive(Debug, serde::Serialize)]
pub struct StreamError {
    pub message: String,
}
impl std::error::Error for StreamError {}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Into<axum::Error> for StreamError {
    fn into(self) -> axum::Error {
        axum::Error::new(self.message)
    }
}

impl From<watcher::Error> for StreamError {
    fn from(e: watcher::Error) -> Self {
        StreamError {
            message: e.to_string(),
        }
    }
}

pub async fn watch_resource<T>(
    client: Client,
) -> impl Stream<Item = Result<ObjectDiff, StreamError>>
where
    T: kube::Resource
        + std::clone::Clone
        + std::marker::Send
        + std::fmt::Debug
        + WatchDiff<T>
        + ExtraInfo<T>
        + for<'de> k8s_openapi::serde::Deserialize<'de>
        + 'static,
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
            cache.entry(current.locator().to_string()).and_modify(|previous| {
                diff = Some(current.diff(previous));
                *previous = current.clone();
            }).or_insert(current);
            if let Some(diff_value) = diff {
                yield Ok(diff_value)
            }
        }
    }
}
