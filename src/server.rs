use std::net::SocketAddr;

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use log::info;
use tokio::net::TcpListener;
use tokio::task::{spawn_local, LocalSet};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::widget_capnp::widget_market;

pub async fn single_market<M: 'static + widget_market::Server>(addr: SocketAddr, market: M) -> Result<(), Box<dyn std::error::Error>> {
    LocalSet::new()
        .run_until(async move {
            let listener = TcpListener::bind(&addr).await?;
            let widget_client: widget_market::Client = capnp_rpc::new_client(market);

            // TODO: we want to also have a trade server
            info!("started server at {}", addr);
            loop {
                let (stream, _) = listener.accept().await?;
                stream.set_nodelay(true)?;
                let (reader, writer) = TokioAsyncReadCompatExt::compat(stream).split();
                let network = twoparty::VatNetwork::new(
                    reader,
                    writer,
                    rpc_twoparty_capnp::Side::Server,
                    Default::default(),
                );
                let rpc_system =
                    RpcSystem::new(Box::new(network), Some(widget_client.clone().client));

                spawn_local(Box::pin(rpc_system.map(|_| ())));
            }
        })
        .await
}
