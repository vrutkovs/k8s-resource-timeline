use crate::stream::watch_with_timeout;

use askama::Template;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};

use serde::{de, Deserialize, Deserializer};
use std::{fmt, str::FromStr};

#[derive(Template)]
#[template(path = "main.html", escape = "none")]
struct TimelineTemplate<'a> {
    intervals: &'a String,
}

/// Serde deserialization decorator to map empty Strings to None,
fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Params {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    timeout: Option<u64>,
}

pub async fn timeline(Query(params): Query<Params>) -> impl IntoResponse {
    let diffs = watch_with_timeout(params.timeout.unwrap_or(60)).await;
    let intervals = serde_json::to_string(&diffs).expect("serizalize to json works");
    let template = TimelineTemplate {
        intervals: &intervals,
    };
    let reply_html = template.render().unwrap();
    (StatusCode::OK, Html(reply_html).into_response())
}
