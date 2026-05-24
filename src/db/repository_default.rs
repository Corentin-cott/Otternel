use std::sync::Mutex;

use log::debug;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

/// Errors returned by the PocketBase-backed [`Database`] layer.
#[derive(Debug, Error)]
pub enum DbError {
    /// Network/transport failure (connection refused, DNS, timeout, ...).
    #[error("HTTP transport error: {0}")]
    Transport(String),

    /// PocketBase replied with a non-2xx status.
    #[error("PocketBase API error (status {status}): {body}")]
    Api { status: u16, body: String },

    /// Superuser authentication failed.
    #[error("authentication failed: {0}")]
    Auth(String),

    /// A response body could not be read or parsed as expected.
    #[error("response decoding error: {0}")]
    Decode(String),

    /// A record expected to exist was not found.
    #[error("record not found")]
    NotFound,
}

#[derive(Deserialize)]
struct AuthResponse {
    token: String,
}

/// PocketBase REST client authenticated as a superuser.
///
/// The auth token is cached and transparently refreshed if the API answers
/// `401` on a request. All record helpers attach the `Authorization` header.
pub struct Database {
    base_url: String,
    admin_email: String,
    admin_password: String,
    token: Mutex<String>,
}

impl Database {
    /// Creates a new client and performs the initial superuser authentication.
    ///
    /// # Parameters
    /// - `base_url`: PocketBase base URL (e.g. `https://pb.example.com`). A
    ///   trailing slash is trimmed.
    /// - `admin_email` / `admin_password`: superuser credentials used for the
    ///   `_superusers` auth-with-password flow.
    ///
    /// # Errors
    /// Returns [`DbError`] if the initial authentication request fails.
    pub fn new(base_url: &str, admin_email: &str, admin_password: &str) -> Result<Self, DbError> {
        let base_url = base_url.trim_end_matches('/').to_string();
        let token = Self::authenticate(&base_url, admin_email, admin_password)?;
        Ok(Self {
            base_url,
            admin_email: admin_email.to_string(),
            admin_password: admin_password.to_string(),
            token: Mutex::new(token),
        })
    }

    /// Performs the `_superusers` auth-with-password flow and returns a token.
    fn authenticate(base_url: &str, email: &str, password: &str) -> Result<String, DbError> {
        let url = format!("{}/api/collections/_superusers/auth-with-password", base_url);
        let resp = ureq::post(&url)
            .send_json(serde_json::json!({ "identity": email, "password": password }));

        match resp {
            Ok(r) => {
                let parsed: AuthResponse =
                    r.into_json().map_err(|e| DbError::Decode(e.to_string()))?;
                Ok(parsed.token)
            }
            Err(ureq::Error::Status(code, r)) => {
                let body = r.into_string().unwrap_or_default();
                Err(DbError::Auth(format!("status {code}: {body}")))
            }
            Err(e) => Err(DbError::Transport(e.to_string())),
        }
    }

    /// Re-authenticates and stores the fresh token.
    fn reauthenticate(&self) -> Result<(), DbError> {
        let new_token =
            Self::authenticate(&self.base_url, &self.admin_email, &self.admin_password)?;
        *self.token.lock().unwrap() = new_token;
        Ok(())
    }

    /// Returns a clone of the current cached token.
    fn token(&self) -> String {
        self.token.lock().unwrap().clone()
    }

    /// Runs `build` with the current token; on a `401`, re-authenticates once
    /// and retries. The closure must rebuild the request on each call because a
    /// `ureq` request is consumed when sent.
    fn send_with_auth<F>(&self, build: F) -> Result<Value, DbError>
    where
        F: Fn(&str) -> Result<ureq::Response, ureq::Error>,
    {
        match build(&self.token()) {
            Ok(resp) => Self::response_to_json(resp),
            Err(ureq::Error::Status(401, _)) => {
                debug!("PocketBase returned 401; re-authenticating");
                self.reauthenticate()?;
                match build(&self.token()) {
                    Ok(resp) => Self::response_to_json(resp),
                    Err(e) => Err(Self::map_ureq_err(e)),
                }
            }
            Err(e) => Err(Self::map_ureq_err(e)),
        }
    }

    /// Reads a response body into JSON, tolerating an empty body (e.g. `204`
    /// on delete) by returning [`Value::Null`].
    fn response_to_json(resp: ureq::Response) -> Result<Value, DbError> {
        let body = resp.into_string().map_err(|e| DbError::Decode(e.to_string()))?;
        if body.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&body).map_err(|e| DbError::Decode(e.to_string()))
    }

    /// Maps a `ureq` error into a [`DbError`], surfacing the HTTP status/body.
    fn map_ureq_err(e: ureq::Error) -> DbError {
        match e {
            ureq::Error::Status(status, r) => {
                let body = r.into_string().unwrap_or_default();
                DbError::Api { status, body }
            }
            other => DbError::Transport(other.to_string()),
        }
    }

    /// Verifies connectivity and that the cached superuser token is accepted.
    ///
    /// Performs an authenticated `GET /api/collections` (superuser-only), so a
    /// success proves both that PocketBase is reachable and that auth works.
    pub fn health_check(&self) -> Result<(), DbError> {
        let url = format!("{}/api/collections", self.base_url);
        self.send_with_auth(|token| {
            ureq::get(&url)
                .set("Authorization", token)
                .query("perPage", "1")
                .call()
        })?;
        Ok(())
    }

    // ===========================
    // Generic record helpers
    // ===========================

    /// Lists records from `collection`. `params` are query parameters passed
    /// straight to PocketBase (e.g. `[("filter", "(server='abc')"), ("perPage", "200")]`);
    /// values are URL-encoded by the HTTP client.
    pub fn list_records(
        &self,
        collection: &str,
        params: &[(&str, &str)],
    ) -> Result<Vec<Value>, DbError> {
        let url = format!("{}/api/collections/{}/records", self.base_url, collection);
        let json = self.send_with_auth(|token| {
            let mut req = ureq::get(&url).set("Authorization", token);
            for (k, v) in params {
                req = req.query(k, v);
            }
            req.call()
        })?;

        let items = json
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(items)
    }

    /// Returns the first record matching `filter`, or `None`.
    pub fn find_first(&self, collection: &str, filter: &str) -> Result<Option<Value>, DbError> {
        let items = self.list_records(collection, &[("filter", filter), ("perPage", "1")])?;
        Ok(items.into_iter().next())
    }

    /// Fetches a single record by id.
    ///
    /// # Errors
    /// Returns [`DbError::NotFound`] if PocketBase answers `404`.
    pub fn get_record(&self, collection: &str, id: &str) -> Result<Value, DbError> {
        let url = format!("{}/api/collections/{}/records/{}", self.base_url, collection, id);
        match self.send_with_auth(|token| ureq::get(&url).set("Authorization", token).call()) {
            Ok(v) => Ok(v),
            Err(DbError::Api { status: 404, .. }) => Err(DbError::NotFound),
            Err(e) => Err(e),
        }
    }

    /// Creates a record from `body` and returns the created record.
    pub fn create_record(&self, collection: &str, body: &Value) -> Result<Value, DbError> {
        let url = format!("{}/api/collections/{}/records", self.base_url, collection);
        self.send_with_auth(|token| {
            ureq::post(&url)
                .set("Authorization", token)
                .send_json(body.clone())
        })
    }

    /// Updates record `id` with `body` and returns the updated record.
    pub fn update_record(
        &self,
        collection: &str,
        id: &str,
        body: &Value,
    ) -> Result<Value, DbError> {
        let url = format!("{}/api/collections/{}/records/{}", self.base_url, collection, id);
        self.send_with_auth(|token| {
            ureq::request("PATCH", &url)
                .set("Authorization", token)
                .send_json(body.clone())
        })
    }

    /// Deletes record `id`.
    pub fn delete_record(&self, collection: &str, id: &str) -> Result<(), DbError> {
        let url = format!("{}/api/collections/{}/records/{}", self.base_url, collection, id);
        self.send_with_auth(|token| {
            ureq::request("DELETE", &url)
                .set("Authorization", token)
                .call()
        })?;
        Ok(())
    }
}
