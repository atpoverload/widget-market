use std::collections::HashMap;
use std::net::SocketAddr;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::AsyncReadExt;
use futures::FutureExt;

use crate::widget_capnp::widget_market;

// can we make this into a impl of some sort so others can use it?
#[derive(Debug)]
pub struct MarketSnapshot {
    pub account: HashMap<String, i32>,
    pub market: HashMap<String, i32>,
}

pub struct WidgetMarketClient {
    service: widget_market::Client,
}

impl WidgetMarketClient {
    pub async fn new(addr: &SocketAddr) -> Result<WidgetMarketClient, Box<dyn std::error::Error>> {
        // set up the rpc system
        let stream = tokio::net::TcpStream::connect(&addr).await?;
        stream.set_nodelay(true)?;
        let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
        let rpc_network = Box::new(twoparty::VatNetwork::new(
            reader,
            writer,
            rpc_twoparty_capnp::Side::Client,
            Default::default(),
        ));
        let mut rpc_system = RpcSystem::new(rpc_network, None);
        let service: widget_market::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        // pin the rpc system to a task
        tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));

        // return the service
        Ok(WidgetMarketClient {service})
    }

    // joins the market and returns the id for the account
    pub async fn join(&self) -> String {
        self.service.join_request().send().promise.await
            .unwrap()
            .get()
            .unwrap()
            .get_id()
            .unwrap()
            .to_string()
    }

    // checks the current status of the market from the account's perspective
    pub async fn check(&self, id: &str) -> MarketSnapshot {
        let mut request = self.service.check_request();
        request.get().set_id(id);

        let result = request.send().promise.await.unwrap();
        let market = result
            .get()
            .unwrap();
        MarketSnapshot {
            account: market.get_account()
                .unwrap()
                .iter()
                .map(|w| (w.get_widget().unwrap().to_string(), w.get_count()))
                .collect(),
            market: market.get_market()
                .unwrap()
                .iter()
                .map(|w| (w.get_widget().unwrap().to_string(), w.get_count()))
                .collect()
        }
    }

    // request a trade be made
    pub async fn trade(&self, id: &str, first: &str, second: &str) -> bool {
        let mut request = self.service.trade_request();
        request.get().set_id(id);
        request.get().set_buy(first);
        request.get().set_sell(second);

        if let Ok(_) = request.send().promise.await {true} else {false}
    }

    // leaves the market and returns the number of points scored
    pub async fn leave(&self, id: &str) -> i32 {
        let mut request = self.service.leave_request();
        request.get().set_id(id);

        request.send().promise.await
            .unwrap()
            .get()
            .unwrap()
            .get_score()
    }
}
