use actix_header::actix_header;
use actix_web::{http::StatusCode, HttpResponse, HttpResponseBuilder};
use serde::Serialize;

pub mod admin;
pub mod auth;
pub mod collection;
pub mod project;
pub mod record;

#[actix_header("Authorization")]
#[derive(Debug)]
pub struct TokenReqHeader(String);

impl TokenReqHeader {
    pub fn get(&self) -> Option<&'_ str> {
        self.0.strip_prefix("Bearer ")
    }
}

impl From<String> for TokenReqHeader {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<TokenReqHeader> for String {
    fn from(s: TokenReqHeader) -> Self {
        s.0
    }
}

#[derive(Serialize)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorRes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pagination: Option<PaginationRes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

impl Response {
    pub fn data<T: Serialize>(
        status_code: StatusCode,
        pagination: Option<PaginationRes>,
        data: T,
    ) -> HttpResponse {
        match serde_json::to_value(data) {
            Ok(data) => HttpResponseBuilder::new(status_code).json(Self {
                error: None,
                pagination,
                data: Some(data),
            }),
            Err(err) => {
                hb_log::error(None, &err);
                Self::error(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
            }
        }
    }

    pub fn error(status_code: StatusCode, message: &str) -> HttpResponse {
        hb_log::error(None, message);

        HttpResponseBuilder::new(status_code).json(Self {
            error: Some(ErrorRes {
                status: match status_code.canonical_reason() {
                    Some(status_code) => status_code.to_owned(),
                    None => "Unknown".to_owned(),
                },
                message: message.to_owned(),
            }),
            pagination: None,
            data: None,
        })
    }
}

#[derive(Serialize)]
pub struct ErrorRes {
    status: String,
    message: String,
}

#[derive(Serialize)]
pub struct PaginationRes {
    limit: i64,
    count: i64,
    page: i64,
    total: i64,
}

impl PaginationRes {
    pub fn new(limit: &i64, count: &i64, page: &i64, total: &i64) -> Self {
        Self {
            limit: *limit,
            count: *count,
            page: *page,
            total: *total,
        }
    }
}
