//! The httpserver capability provider allows wasmcloud actors to receive
//! and process http(s) messages from web browsers, command-line tools
//! such as curl, and other http clients. The server is fully asynchronous,
//! and built on Rust's high-performance warp engine, which is in turn based
//! on hyper, and can process a large number of simultaneous connections.
//!
//! ## Features:
//!
//! - HTTP/1 and HTTP/2
//! - TLS
//! - CORS support (select allowed_origins, allowed_methods,
//!   allowed_headers.) Cors has sensible defaults so it should
//!   work as-is for development purposes, and may need refinement
//!   for production if a more secure configuration is required.
//! - All settings can be specified at runtime, using per-actor link settings:
//!   - bind interface/port
//!   - logging level
//!   - TLS
//!   - Cors
//! - Flexible confiuration loading: from host, or from local toml or json file.
//! - Fully asynchronous, using tokio lightweight "green" threads
//! - Thread pool (for managing a pool of OS threads). The default
//!   thread pool has one thread per cpu core.
//! - Packaged as a rust library crate for implementation flexibility
//!
//! ## More tech info:
//!
//! Each actor that links to this provider gets
//! its own bind address (interface ip and port) and a lightweight
//! tokio thread (lighter weight than an OS thread, more like "green threads").
//! Tokio can manage a thread pool (of OS threads) to be shared
//! by the all of the server green threads.
//!

// TODO: These types should be defined via WIT
pub mod wasmcloud_interface_httpserver;

mod hashmap_ci;
mod settings;

pub use settings::{load_settings, ServiceSettings, CONTENT_LEN_LIMIT, DEFAULT_MAX_CONTENT_LEN};
pub use wasmcloud_interface_httpserver::{
    HttpRequest as Request, HttpResponse as Response, HttpServer as Server,
    HttpServerSender as ServerSender,
};

pub(crate) use hashmap_ci::make_case_insensitive;

use std::collections::HashMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use flume::{bounded, Receiver, Sender};
use futures::Future;
use http::header::HeaderMap;

use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace, Instrument};
use warp::{filters::cors::Builder, path::FullPath, Filter};
use wasmbus_rpc::{core::LinkDefinition, error::RpcResult};

/// errors generated by this crate
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("problem reading settings: {0}")]
    Settings(String),

    #[error("provider startup: {0}")]
    Init(String),

    #[error("warp error: {0}")]
    Warp(warp::Error),

    #[error("deserializing settings: {0}")]
    SettingsToml(toml::de::Error),
}

pub type AsyncCallActorFn = Box<
    dyn Fn(
            String,
            Arc<LinkDefinition>,
            Request,
            Option<Duration>,
        ) -> Pin<Box<dyn Future<Output = RpcResult<Response>> + Send + 'static>>
        + Send
        + Sync,
>;

struct CallActorFn(AsyncCallActorFn);

pub struct Inner {
    settings: ServiceSettings,
    lattice_id: String,
    shutdown_tx: Sender<bool>,
    shutdown_rx: Receiver<bool>,
    call_actor: CallActorFn,
}

/// An asynchronous HttpServer with support for CORS and TLS
/// ```no_test
///   use wasmcloud_provider_httpserver::{HttpServer, load_settings};
///   let settings = load_settings(ld.values)?;
///   let server = HttpServer::new(settings);
///   let task = server.serve()?;
///   tokio::task::spawn(task);
/// ```
#[derive(Clone)]
pub struct HttpServerCore {
    inner: Arc<Inner>,
}

impl std::ops::Deref for HttpServerCore {
    type Target = Inner;
    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl HttpServerCore {
    /// Initializes server with settings
    pub fn new<F, Fut>(settings: ServiceSettings, lattice_id: String, call_actor_fn: F) -> Self
    where
        F: Fn(String, Arc<LinkDefinition>, Request, Option<Duration>) -> Fut
            + Send
            + Sync
            + 'static,
        Fut: Future<Output = RpcResult<Response>> + 'static + Send,
    {
        let (shutdown_tx, shutdown_rx) = bounded(1);
        let call_actor_fn = Arc::new(call_actor_fn);
        Self {
            inner: Arc::new(Inner {
                settings,
                lattice_id,
                shutdown_tx,
                shutdown_rx,
                call_actor: CallActorFn(Box::new(
                    move |lattice: String,
                          ld: Arc<LinkDefinition>,
                          req: Request,
                          timeout: Option<Duration>| {
                        let call_actor_fn = call_actor_fn.clone();
                        Box::pin(call_actor_fn(lattice, ld, req, timeout))
                    },
                )),
            }),
        }
    }

    /// Initiate server shutdown. This can be called from any thread and is non-blocking.
    pub fn begin_shutdown(&self) {
        let _ = self.shutdown_tx.try_send(true);
    }

    /// Start the server in a new thread
    /// ```no_test
    ///    use wasmcloud_provider_httpserver::{HttpServer, load_settings};
    ///    let settings = load_settings(&ld.values)?;
    ///    let server = HttpServer::new(settings);
    ///    let _ = server.start().await?;
    /// ```
    pub async fn start(&self, ld: &LinkDefinition) -> Result<JoinHandle<()>, Error> {
        let timeout = self
            .inner
            .settings
            .timeout_ms
            .map(std::time::Duration::from_millis);

        let ld = Arc::new(ld.clone());
        let linkdefs = ld.clone();
        let trace_ld = ld.clone();
        let arc_inner = self.inner.clone();
        let route = warp::any()
            .and(warp::header::headers_cloned())
            .and(warp::method())
            .and(warp::body::bytes())
            .and(warp::path::full())
            .and(opt_raw_query())
            .and_then(
                move |
                      headers: HeaderMap,
                      method: http::method::Method,
                      body: Bytes,
                      path: FullPath,
                      query: String| {
                    let span = tracing::debug_span!("http request", %method, path = %path.as_str(), %query);
                    let ld = linkdefs.clone();
                    let arc_inner = arc_inner.clone();
                    async move{
                        if let Some(readonly_mode) = arc_inner.settings.readonly_mode{
                            if readonly_mode && method!= http::method::Method::GET && method!= http::method::Method::HEAD {
                                debug!("Cannot use other methods in Read Only Mode");
                                // If this fails it is developer error, so unwrap is okay
                                let resp = http::Response::builder().status(http::StatusCode::METHOD_NOT_ALLOWED).body(Vec::with_capacity(0)).unwrap();
                                return Ok::<_, warp::Rejection>(resp)
                            }
                        }
                        let hmap = convert_request_headers(&headers);
                        let req = Request {
                            body: Vec::from(body),
                            header: hmap,
                            method: method.as_str().to_ascii_uppercase(),
                            path: path.as_str().to_string(),
                            query_string: query,
                        };
                        trace!(
                            ?req,
                            "httpserver calling actor"
                        );
                        let response = match arc_inner.call_actor.call(arc_inner.lattice_id.clone(), ld.clone(), req, timeout).in_current_span().await {
                            Ok(resp) => resp,
                            Err(e) => {
                                error!(
                                    error = %e,
                                    "Error sending Request to actor"
                                );
                                Response {
                                    status_code: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                                    body: Default::default(),
                                    header: Default::default(),
                                }
                            }
                        };
                        let status = match http::StatusCode::from_u16(response.status_code) {
                            Ok(status_code) => status_code,
                            Err(e) => {
                                error!(
                                    status_code = %response.status_code,
                                    error = %e,
                                    "invalid response status code, changing to 500"
                                );
                                http::StatusCode::INTERNAL_SERVER_ERROR
                            }
                        };
                        let http_builder = http::Response::builder()
                        .status(status);
                        let http_builder = if let Some(cache_control_header) = arc_inner.settings.cache_control.as_ref(){
                            let mut builder = http_builder;
                            builder = builder.header("Cache-Control",cache_control_header);
                            builder
                        }else{
                            http_builder
                        };
                        // Unwrapping here because validation takes place for the linkdef
                        let mut http_response = http_builder.body(response.body).unwrap();
                        convert_response_headers(response.header, http_response.headers_mut());
                        Ok::<_, warp::Rejection>(http_response)
                    }.instrument(span)
                },
            ).with(warp::trace(move |req_info| {
                let actor_id = &trace_ld.actor_id;
                let span = tracing::debug_span!("request", method = %req_info.method(), path = %req_info.path(), query = tracing::field::Empty, %actor_id);
                if let Some(remote_addr) = req_info.remote_addr() {
                    span.record("remote_addr", &tracing::field::display(remote_addr));
                }

                span
            }));

        let addr = self.settings.address.unwrap();
        info!(
            %addr,
            actor_id = %ld.actor_id,
            "httpserver starting listener for actor",
        );

        // add Cors configuration, if enabled, and spawn either TlsServer or Server
        let cors = cors_filter(&self.settings)?;
        let server = warp::serve(route.with(cors));
        let handle = tokio::runtime::Handle::current();
        let shutdown_rx = self.shutdown_rx.clone();
        let join = if self.settings.tls.is_set() {
            let (_, fut) = server
                .tls()
                // unwrap ok here because tls.is_set confirmed both fields are some()
                .key_path(self.settings.tls.priv_key_file.as_ref().unwrap())
                .cert_path(self.settings.tls.cert_file.as_ref().unwrap())
                // we'd prefer to use try_bind_with_graceful_shutdown but it's not supported
                // for tls server yet. Waiting on https://github.com/seanmonstar/warp/pull/717
                // attempt to bind to the address
                .bind_with_graceful_shutdown(addr, async move {
                    if let Err(e) = shutdown_rx.recv_async().await {
                        error!(error = %e, "shutting down httpserver listener");
                    }
                });
            handle.spawn(fut)
        } else {
            let (_, fut) = server
                .try_bind_with_graceful_shutdown(addr, async move {
                    if let Err(e) = shutdown_rx.recv_async().await {
                        error!(error = %e, "shutting down httpserver listener");
                    }
                })
                .map_err(|e| {
                    Error::Settings(format!(
                        "failed binding to address '{}' reason: {}",
                        &addr.to_string(),
                        e
                    ))
                })?;
            handle.spawn(fut)
        };

        Ok(join)
    }
}

impl Drop for HttpServerCore {
    /// drop the client connection. Does not block or fail if the client has already been closed.
    fn drop(&mut self) {
        let _ = self.shutdown_tx.try_send(true);
    }
}

/// convert request headers from incoming warp server to HeaderMap
fn convert_request_headers(headers: &http::HeaderMap) -> HashMap<String, Vec<String>> {
    let mut hmap = HashMap::default();
    for k in headers.keys() {
        let vals = headers
            .get_all(k)
            .iter()
            // from http crate:
            //    In practice, HTTP header field values are usually valid ASCII.
            //     However, the HTTP spec allows for a header value to contain
            //     opaque bytes as well.
            // This implementation only forwards headers with ascii values to the actor.
            .filter_map(|val| val.to_str().ok())
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        if !vals.is_empty() {
            hmap.insert(k.to_string(), vals);
        }
    }
    hmap
}

/// convert HeaderMap from actor into warp's HeaderMap for returning to http client
fn convert_response_headers(
    header: HashMap<String, Vec<String>>,
    headers_mut: &mut http::header::HeaderMap,
) {
    let map = headers_mut;
    for (k, vals) in header.into_iter() {
        let name = match http::header::HeaderName::from_bytes(k.as_bytes()) {
            Ok(name) => name,
            Err(e) => {
                error!(
                    header_name = %k,
                    error = %e,
                    "invalid response header name, sending without this header"
                );
                continue;
            }
        };
        map.remove(&name);
        for val in vals.into_iter() {
            let value = match http::header::HeaderValue::try_from(val) {
                Ok(value) => value,
                Err(e) => {
                    error!(
                        error = %e,
                        "Non-ascii header value, skipping this header"
                    );
                    continue;
                }
            };
            map.append(&name, value);
        }
    }
}

/// get raw query as string or optional query
fn opt_raw_query() -> impl Filter<Extract = (String,), Error = Infallible> + Copy {
    warp::any().and(
        warp::filters::query::raw()
            .or(warp::any().map(String::default))
            .unify(),
    )
}

/// build warp Cors filter from settings
fn cors_filter(settings: &settings::ServiceSettings) -> Result<warp::filters::cors::Cors, Error> {
    let mut cors: Builder = warp::cors();

    match settings.cors.allowed_origins {
        Some(ref allowed_origins) if !allowed_origins.is_empty() => {
            cors = cors.allow_origins(allowed_origins.iter().map(AsRef::as_ref));
        }
        _ => {
            cors = cors.allow_any_origin();
        }
    }

    if let Some(ref allowed_headers) = settings.cors.allowed_headers {
        cors = cors.allow_headers(allowed_headers.iter());
    }
    if let Some(ref allowed_methods) = settings.cors.allowed_methods {
        for m in allowed_methods.iter() {
            match http::method::Method::try_from(m.as_str()) {
                Err(_) => return Err(Error::InvalidParameter(format!("method: '{}'", m))),
                Ok(method) => {
                    cors = cors.allow_method(method);
                }
            }
        }
    }

    if let Some(ref exposed_headers) = settings.cors.exposed_headers {
        cors = cors.expose_headers(exposed_headers.iter());
    }

    if let Some(max_age) = settings.cors.max_age_secs {
        cors = cors.max_age(std::time::Duration::from_secs(max_age));
    }
    Ok(cors.build())
}

/// Convert setting for max content length of form '[0-9]+(g|G|m|M|k|K)?'
/// Empty string is accepted and returns the default value (currently '10M')
pub fn convert_human_size(value: &str) -> Result<u64, Error> {
    let value = value.trim();
    let mut limit = None;
    if value.is_empty() {
        limit = Some(DEFAULT_MAX_CONTENT_LEN);
    } else if let Ok(num) = value.parse::<u64>() {
        limit = Some(num);
    } else {
        let (num, units) = value.split_at(value.len() - 1);
        if let Ok(base_value) = num.trim().parse::<u64>() {
            match units {
                "k" | "K" => {
                    limit = Some(base_value * 1024);
                }
                "m" | "M" => {
                    limit = Some(base_value * 1024 * 1024);
                }
                "g" | "G" => {
                    limit = Some(base_value * 1024 * 1024 * 1024);
                }
                _ => {}
            }
        }
    }
    match limit {
        Some(x) if x > 0 && x <= CONTENT_LEN_LIMIT => Ok(x),
        Some(_) => {
            Err(Error::Settings(format!("Invalid size in max_content_len '{}': value must be >0 and <= {}", value, settings::CONTENT_LEN_LIMIT)))
        }
        None => {
            Err(Error::Settings(format!("Invalid size in max_content_len: '{}'. Should be a number, optionally followed by 'K', 'M', or 'G'. Example: '10M'. Value must be <= i32::MAX", value)))
        }
    }
}

impl CallActorFn {
    fn call(
        &self,
        lattice_id: String,
        ld: Arc<LinkDefinition>,
        req: Request,
        timeout: Option<Duration>,
    ) -> Pin<Box<dyn Future<Output = RpcResult<Response>> + Send + 'static>> {
        Box::pin((self.0.as_ref())(lattice_id, ld, req, timeout))
    }
}

#[test]
fn parse_max_content_len() {
    // emtpy string returns default
    assert_eq!(convert_human_size("").unwrap(), DEFAULT_MAX_CONTENT_LEN);
    // simple number
    assert_eq!(convert_human_size("4").unwrap(), 4);
    assert_eq!(convert_human_size("12345678").unwrap(), 12345678);
    // k, K, m, M, g, G suffix
    assert_eq!(convert_human_size("2k").unwrap(), 2 * 1024);
    assert_eq!(convert_human_size("2K").unwrap(), 2 * 1024);
    assert_eq!(convert_human_size("10m").unwrap(), 10 * 1024 * 1024);
    assert_eq!(convert_human_size("10M").unwrap(), 10 * 1024 * 1024);

    // allow space before units
    assert_eq!(convert_human_size("10 M").unwrap(), 10 * 1024 * 1024);

    // remove surrounding white space
    assert_eq!(convert_human_size(" 5 k ").unwrap(), 5 * 1024);

    // errors
    assert!(convert_human_size("k").is_err());
    assert!(convert_human_size("0").is_err());
    assert!(convert_human_size("1mb").is_err());
    assert!(convert_human_size(&i32::MAX.to_string()).is_err());
    assert!(convert_human_size(&(i32::MAX as u64 + 1).to_string()).is_err());
}
