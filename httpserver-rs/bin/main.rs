//! Http Server implementation for wasmcloud:httpserver
//!
//!
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use tokio::sync::RwLock;
use wasmbus_rpc::{core::LinkDefinition, provider::prelude::*};
use wasmcloud_provider_httpserver::{load_settings, HttpServer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // handle lattice control messages and forward rpc to the provider dispatch
    // returns when provider receives a shutdown control message
    provider_main(HttpServerProvider::default())?;

    eprintln!("HttpServer provider exiting");
    Ok(())
}

/// HttpServer provider implementation.
#[derive(Default, Clone, Provider)]
struct HttpServerProvider {
    // map to store http server (and its link parameters) for each linked actor
    actors: Arc<RwLock<HashMap<String, HttpServer>>>,
}
/// use default implementations of provider methods
impl ProviderDispatch for HttpServerProvider {}

/// Your provider can handle any of these methods
/// to receive notification of new actor links, deleted links,
/// and for handling health check.
/// Default handlers are implemented in the trait ProviderHandler.
#[async_trait]
impl ProviderHandler for HttpServerProvider {
    /// Provider should perform any operations needed for a new link,
    /// including setting up per-actor resources, and checking authorization.
    /// If the link is allowed, return true, otherwise return false to deny the link.
    async fn put_link(&self, ld: &LinkDefinition) -> Result<bool, RpcError> {
        let settings =
            load_settings(&ld.values).map_err(|e| RpcError::ProviderInit(e.to_string()))?;

        let http_server = HttpServer::new(settings.clone(), get_host_bridge());
        http_server.start(ld.clone()).await.map_err(|e| {
            RpcError::ProviderInit(format!(
                "starting httpserver for {} {:?}: {}",
                &ld.actor_id,
                &settings.address,
                e.to_string()
            ))
        })?;

        let mut update_map = self.actors.write().await;
        update_map.insert(ld.actor_id.to_string(), http_server);

        Ok(true)
    }

    /// Handle notification that a link is dropped - stop the http listener
    async fn delete_link(&self, actor_id: &str) {
        let mut aw = self.actors.write().await;
        if let Some(server) = aw.remove(actor_id) {
            log::info!("httpserver stopping listener for actor {}", actor_id);
            server.begin_shutdown().await;
        }
    }

    /// Handle shutdown request by shutting down all the http server threads
    async fn shutdown(&self) -> Result<(), Infallible> {
        let mut aw = self.actors.write().await;
        // empty the actor link data and stop all servers
        for (_, server) in aw.drain() {
            server.begin_shutdown().await;
        }
        Ok(())
    }
}
