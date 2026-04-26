use std::time::Duration;

use anyhow::Error;
use axum::extract::Query;
use cached::proc_macro::io_cached;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::error;

const F1_STATIC: &str = "https://livetiming.formula1.com/static";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSession {
    key: i64,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    path: String,
    start_date: String,
    end_date: String,
    gmt_offset: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveMeeting {
    key: i64,
    name: String,
    official_name: String,
    location: String,
    country_code: String,
    country_name: String,
    sessions: Vec<ArchiveSession>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawIndex {
    meetings: Vec<RawMeeting>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawMeeting {
    key: i64,
    name: String,
    #[serde(default)]
    official_name: String,
    location: String,
    country: RawCountry,
    sessions: Vec<RawSession>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawCountry {
    code: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawSession {
    key: i64,
    name: String,
    #[serde(rename = "Type")]
    kind: String,
    path: String,
    start_date: String,
    end_date: String,
    gmt_offset: String,
}

async fn fetch_json_stripped(url: &str) -> Result<String, Error> {
    let body = reqwest::get(url).await?.error_for_status()?.text().await?;
    Ok(body.trim_start_matches('\u{feff}').to_string())
}

#[io_cached(
    map_error = r##"|e| anyhow::anyhow!(format!("disk cache error {:?}", e))"##,
    disk = true,
    time = 1800,
    key = "i32",
    convert = r##"{ year }"##
)]
async fn get_archive(year: i32) -> Result<Vec<ArchiveMeeting>, Error> {
    let url = format!("{F1_STATIC}/{year}/Index.json");
    let body = fetch_json_stripped(&url).await?;
    let raw: RawIndex = serde_json::from_str(&body)?;

    let meetings = raw
        .meetings
        .into_iter()
        .map(|m| ArchiveMeeting {
            key: m.key,
            name: m.name,
            official_name: m.official_name,
            location: m.location,
            country_code: m.country.code,
            country_name: m.country.name,
            sessions: m
                .sessions
                .into_iter()
                .map(|s| ArchiveSession {
                    key: s.key,
                    name: s.name,
                    kind: s.kind,
                    path: s.path,
                    start_date: s.start_date,
                    end_date: s.end_date,
                    gmt_offset: s.gmt_offset,
                })
                .collect(),
        })
        .collect();

    Ok(meetings)
}

#[io_cached(
    map_error = r##"|e| anyhow::anyhow!(format!("disk cache error {:?}", e))"##,
    disk = true,
    time = 1800,
    key = "String",
    convert = r##"{ path.clone() }"##
)]
async fn get_session_feeds(path: String) -> Result<Value, Error> {
    let url = format!("{F1_STATIC}/{path}Index.json");
    let body = fetch_json_stripped(&url).await?;
    let value: Value = serde_json::from_str(&body)?;
    Ok(value)
}

#[derive(Deserialize)]
pub struct ArchiveQuery {
    year: Option<i32>,
}

pub async fn get(
    Query(q): Query<ArchiveQuery>,
) -> Result<axum::Json<Vec<ArchiveMeeting>>, axum::http::StatusCode> {
    let year = q.year.unwrap_or_else(|| chrono::Utc::now().year());

    match get_archive(year).await {
        Ok(meetings) => Ok(axum::Json(meetings)),
        Err(err) => {
            error!(?err, year, "failed to load archive index");
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize)]
pub struct SessionQuery {
    path: String,
}

pub async fn get_session(
    Query(q): Query<SessionQuery>,
) -> Result<axum::Json<Value>, axum::http::StatusCode> {
    if q.path.contains("..") || q.path.starts_with('/') {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }

    match get_session_feeds(q.path.clone()).await {
        Ok(v) => Ok(axum::Json(v)),
        Err(err) => {
            error!(?err, path = %q.path, "failed to load session feeds");
            Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
