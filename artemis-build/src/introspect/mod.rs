use artemis::QueryBody;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use std::{
    env,
    error::Error,
    fmt,
    fs::File,
    path::{Path, PathBuf}
};

mod query;

#[derive(Debug)]
pub enum IntrospectionError {
    RemoteError(String),
    SerializationError(serde_json::Error),
    DeserializationError(reqwest::Error),
    NetworkError(reqwest::Error),
    IoError(std::io::Error),
    ArgumentError(String)
}
impl Error for IntrospectionError {}
impl fmt::Display for IntrospectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntrospectionError::RemoteError(msg) => write!(f, "server status not OK: {}", msg),
            IntrospectionError::SerializationError(e) => write!(f, "serialization error: {}", e),
            IntrospectionError::DeserializationError(e) => write!(f, "serialization error: {}", e),
            IntrospectionError::NetworkError(e) => write!(f, "network error: {}", e),
            IntrospectionError::IoError(e) => write!(f, "io error: {}", e),
            IntrospectionError::ArgumentError(msg) => write!(f, "invalid argument: {}", msg)
        }
    }
}

pub struct Header {
    key: &'static str,
    value: &'static str
}

pub(crate) fn introspect(
    location: &str,
    authorization: Option<String>,
    headers: Vec<Header>
) -> Result<PathBuf, IntrospectionError> {
    let out_dir = env::var("OUT_DIR")
        .map_err(|_| IntrospectionError::ArgumentError("OUT_DIR must be set.".to_string()))?;

    let path: &Path = out_dir.as_str().as_ref();

    let file_path = path.join("schema.json");
    let out = File::create(file_path.clone()).map_err(IntrospectionError::IoError)?;

    let request_body = QueryBody {
        query: query::QUERY,
        operation_name: query::OPERATION_NAME,
        variables: query::Variables
    };

    let client = reqwest::blocking::Client::new();
    let mut req_builder = client.post(location).headers(construct_headers(headers));

    if let Some(token) = authorization {
        req_builder = req_builder.bearer_auth(token.as_str())
    }

    let res = req_builder
        .json(&request_body)
        .send()
        .map_err(IntrospectionError::NetworkError)?;

    if res.status().is_success() {
        let json: serde_json::Value = res
            .json()
            .map_err(IntrospectionError::DeserializationError)?;
        serde_json::to_writer_pretty(out, &json).map_err(IntrospectionError::SerializationError)?;
        Ok(file_path)
    } else if res.status().is_server_error() {
        Err(IntrospectionError::RemoteError("server error!".to_string()))
    } else {
        Err(IntrospectionError::RemoteError(format!(
            "Status: {}",
            res.status()
        )))
    }
}

fn construct_headers(extra_headers: Vec<Header>) -> HeaderMap {
    let mut headers = HeaderMap::new();

    // insert default headers
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    for header in extra_headers {
        headers.insert(header.key, HeaderValue::from_static(header.value));
    }

    headers
}
