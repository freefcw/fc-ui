#[cfg(feature = "http")]
use futures::{future::BoxFuture, AsyncReadExt, FutureExt};
#[cfg(feature = "http")]
use gpui::http_client::{AsyncBody, HttpClient, RedirectPolicy, Request, RequestTimeout, Response};
#[cfg(feature = "http")]
use std::{
    fmt,
    sync::{Arc, OnceLock},
};

/// User agent used by the built-in HTTP client.
pub const DEFAULT_USER_AGENT: &str = concat!("fc-ui/", env!("CARGO_PKG_VERSION"));

/// Controls how root initialization handles GPUI's application-wide HTTP client.
#[cfg(feature = "http")]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum HttpSetup {
    /// Install the built-in client with [`DEFAULT_USER_AGENT`].
    #[default]
    Default,
    /// Install the built-in client with the supplied user agent.
    UserAgent(String),
    /// Leave GPUI's current HTTP client unchanged.
    PreserveExisting,
}

/// Error returned when the built-in HTTP client cannot be constructed.
#[cfg(feature = "http")]
#[derive(Debug)]
pub struct HttpInitError {
    source: reqwest::Error,
}

#[cfg(feature = "http")]
impl fmt::Display for HttpInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to initialize HTTP client: {}",
            self.source
        )
    }
}

#[cfg(feature = "http")]
impl std::error::Error for HttpInitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[cfg(feature = "http")]
impl From<reqwest::Error> for HttpInitError {
    fn from(source: reqwest::Error) -> Self {
        Self { source }
    }
}

#[cfg(feature = "http")]
pub struct SimpleHttpClient {
    client: reqwest::blocking::Client,
    no_redirect_client: OnceLock<reqwest::blocking::Client>,
    user_agent_string: String,
    user_agent: gpui::http_client::http::HeaderValue,
}

#[cfg(feature = "http")]
impl SimpleHttpClient {
    fn build_client(
        user_agent: &str,
        redirects: reqwest::redirect::Policy,
    ) -> Result<reqwest::blocking::Client, HttpInitError> {
        reqwest::blocking::Client::builder()
            .user_agent(user_agent)
            .redirect(redirects)
            .build()
            .map_err(HttpInitError::from)
    }

    fn no_redirect_client(&self) -> Result<&reqwest::blocking::Client, HttpInitError> {
        if let Some(client) = self.no_redirect_client.get() {
            return Ok(client);
        }
        let client =
            Self::build_client(&self.user_agent_string, reqwest::redirect::Policy::none())?;
        Ok(self.no_redirect_client.get_or_init(|| client))
    }

    pub fn new(user_agent: &str) -> Result<Arc<Self>, HttpInitError> {
        let client = Self::build_client(user_agent, reqwest::redirect::Policy::default())?;

        let user_agent_header = gpui::http_client::http::HeaderValue::from_str(user_agent)
            .unwrap_or_else(|_| gpui::http_client::http::HeaderValue::from_static("fc-ui"));

        Ok(Arc::new(Self {
            client,
            no_redirect_client: OnceLock::new(),
            user_agent_string: user_agent.to_string(),
            user_agent: user_agent_header,
        }))
    }

    pub fn with_default_user_agent() -> Result<Arc<Self>, HttpInitError> {
        Self::new(DEFAULT_USER_AGENT)
    }

    fn failed_request(
        error: HttpInitError,
    ) -> BoxFuture<'static, gpui::http_client::Result<Response<AsyncBody>>> {
        async move { Err(error.into()) }.boxed()
    }
}

#[cfg(feature = "http")]
async fn read_request_body(mut body: AsyncBody) -> gpui::http_client::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    body.read_to_end(&mut bytes)
        .await
        .map_err(|e| gpui::http_client::anyhow!("Failed to read request body: {}", e))?;
    Ok(bytes)
}

#[cfg(feature = "http")]
impl HttpClient for SimpleHttpClient {
    fn type_name(&self) -> &'static str {
        "SimpleHttpClient"
    }

    fn user_agent(&self) -> Option<&gpui::http_client::http::HeaderValue> {
        Some(&self.user_agent)
    }

    fn proxy(&self) -> Option<&gpui::http_client::Url> {
        None
    }

    fn send(
        &self,
        req: Request<AsyncBody>,
    ) -> BoxFuture<'static, gpui::http_client::Result<Response<AsyncBody>>> {
        let (parts, body) = req.into_parts();
        let redirect_policy = parts
            .extensions
            .get::<RedirectPolicy>()
            .cloned()
            .unwrap_or_default();
        let timeout = parts.extensions.get::<RequestTimeout>().map(|t| t.0);

        let client = match &redirect_policy {
            RedirectPolicy::NoFollow => match self.no_redirect_client() {
                Ok(client) => client.clone(),
                Err(error) => return Self::failed_request(error),
            },
            RedirectPolicy::FollowAll => self.client.clone(),
            // `FollowLimit` arity is unbounded, so it cannot share a cached client.
            RedirectPolicy::FollowLimit(limit) => {
                match Self::build_client(
                    &self.user_agent_string,
                    reqwest::redirect::Policy::limited(*limit as usize),
                ) {
                    Ok(client) => client,
                    Err(error) => return Self::failed_request(error),
                }
            }
        };

        async move {
            let body_bytes = read_request_body(body).await?;
            let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())
                .map_err(|e| gpui::http_client::anyhow!("Unsupported HTTP method: {}", e))?;
            let uri_str = parts.uri.to_string();
            let headers: Vec<_> = parts
                .headers
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();

            smol::unblock(move || {
                let mut request_builder = client.request(method, &uri_str);

                for (key, value) in headers {
                    request_builder = request_builder.header(key, value);
                }

                if let Some(timeout) = timeout {
                    request_builder = request_builder.timeout(timeout);
                }

                if !body_bytes.is_empty() {
                    request_builder = request_builder.body(body_bytes);
                }

                let response = request_builder
                    .send()
                    .map_err(|e| gpui::http_client::anyhow!("HTTP request failed: {}", e))?;

                let status = response.status();
                let headers = response.headers().clone();
                let bytes = response.bytes().map_err(|e| {
                    gpui::http_client::anyhow!("Failed to read response body: {}", e)
                })?;

                let mut builder = gpui::http_client::http::Response::builder().status(
                    gpui::http_client::http::StatusCode::from_u16(status.as_u16())
                        .map_err(|e| gpui::http_client::anyhow!("Invalid status code: {}", e))?,
                );

                for (key, value) in headers.iter() {
                    builder = builder.header(key.as_str(), value.as_bytes());
                }

                let async_body = AsyncBody::from_bytes(bytes);
                let response = builder
                    .body(async_body)
                    .map_err(|e| gpui::http_client::anyhow!("Failed to build response: {}", e))?;

                Ok(response)
            })
            .await
        }
        .boxed()
    }
}

#[cfg(feature = "http")]
pub(crate) fn try_init_http_with_setup(
    cx: &mut gpui::App,
    setup: HttpSetup,
) -> Result<(), HttpInitError> {
    let client = match setup {
        HttpSetup::Default => SimpleHttpClient::with_default_user_agent()?,
        HttpSetup::UserAgent(user_agent) => SimpleHttpClient::new(&user_agent)?,
        HttpSetup::PreserveExisting => return Ok(()),
    };

    cx.set_http_client(client);
    Ok(())
}

#[cfg(feature = "http")]
/// Install the built-in HTTP client with [`DEFAULT_USER_AGENT`].
pub fn try_init_http(cx: &mut gpui::App) -> Result<(), HttpInitError> {
    try_init_http_with_setup(cx, HttpSetup::Default)
}

#[cfg(feature = "http")]
/// Install the built-in HTTP client with a caller-supplied user agent.
pub fn try_init_http_with_user_agent(
    cx: &mut gpui::App,
    user_agent: &str,
) -> Result<(), HttpInitError> {
    try_init_http_with_setup(cx, HttpSetup::UserAgent(user_agent.into()))
}

#[cfg(feature = "http")]
fn report_initialization_error(error: &HttpInitError) {
    eprintln!("fc-ui: {error}");
}

pub fn init_http(cx: &mut gpui::App) {
    #[cfg(feature = "http")]
    if let Err(error) = try_init_http(cx) {
        report_initialization_error(&error);
    }
    #[cfg(not(feature = "http"))]
    let _ = cx;
}

pub fn init_http_with_user_agent(cx: &mut gpui::App, user_agent: &str) {
    #[cfg(feature = "http")]
    if let Err(error) = try_init_http_with_user_agent(cx, user_agent) {
        report_initialization_error(&error);
    }
    #[cfg(not(feature = "http"))]
    {
        let _ = cx;
        let _ = user_agent;
    }
}
