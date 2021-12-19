// a simple server that runs a single market server an queries the underlying market on the caller thread
use std::net::SocketAddr;
use std::collections::HashMap;

use capnp_rpc::pry;
use capnp::capability::Promise;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use log::{error, info};
use tokio::net::TcpListener;
use tokio::task::{spawn_local, LocalSet};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::market::Market;
use crate::widget_capnp;

impl <M: Market> widget_capnp::market::Server for M {
    fn join(&mut self, params: widget_capnp::market::JoinParams, mut results: widget_capnp::market::JoinResults) -> Promise<(), capnp::Error> {
        info!("join requested");

        let request = pry!(params.get());
        if request.has_account() {
            let account: HashMap<String, i32> = request
                .get_account()
                .unwrap()
                .iter()
                .map(|c| (c.get_widget().unwrap().to_string(), c.get_count()))
                .collect();
            match self.add_account(account) {
                Ok(id) => {
                    info!("added account {}", id);
                    results.get().set_id(&id);
                    Promise::ok(())
                }
                Err(error) => {
                    error!("unable to add account");
                    error!("{:?}", error);
                    Promise::err(capnp::Error::failed(format!("{:?}", error)))
                }
            }
        } else {
            match self.create_account() {
                Ok(id) => {
                    info!("created account {}", id);
                    results.get().set_id(&id);
                    Promise::ok(())
                }
                Err(error) => {
                    error!("unable to create account");
                    error!("{:?}", error);
                    Promise::err(capnp::Error::failed(format!("{:?}", error)))
                }
            }
        }
    }

    fn check(&mut self, params: widget_capnp::market::CheckParams, mut results: widget_capnp::market::CheckResults) -> Promise<(), capnp::Error> {
        let id = pry!(params.get()).get_id().unwrap();
        info!("check requested by account {}", id);

        match self.get_market() {
            Ok(market) => match self.get_account(id) {
                Ok(account) => {
                    let mut results = results.get();
                    let mut builder = results.reborrow().init_market(market.len() as u32);
                    market.iter().enumerate().for_each(|(i, (w, c))| {
                        builder.reborrow().get(i as u32).set_widget(w);
                        builder.reborrow().get(i as u32).set_count(*c as i32);
                    });

                    let mut builder = results.reborrow().init_account(market.len() as u32);
                    account.iter().enumerate().for_each(|(i, (w, c))| {
                        builder.reborrow().get(i as u32).set_widget(w);
                        builder.reborrow().get(i as u32).set_count(*c as i32);
                    });
                    Promise::ok(())
                }
                Err(error) => {
                    error!("unable to get account {}", id);
                    error!("{:?}", error);
                    Promise::err(capnp::Error::failed(format!("{:?}", error)))
                }
            },
            Err(error) => {
                error!("unable to get market");
                error!("{:?}", error);
                Promise::err(capnp::Error::failed(format!("{:?}", error)))
            }
        }
    }

    fn trade(&mut self, params: widget_capnp::market::TradeParams, _: widget_capnp::market::TradeResults) -> Promise<(), capnp::Error> {
        // grab the params
        let params = pry!(params.get());
        let id = params.get_id().unwrap();
        let buy = params.get_buy().unwrap();
        let sell = params.get_sell().unwrap();
        info!("trade of {} -> {} requested by account {}", buy, sell, id);

        match self.submit_trade(id, buy, sell) {
            Ok(()) => Promise::ok(()),
            Err(error) => {
                error!("unable to make trade");
                error!("{:?}", error);
                Promise::err(capnp::Error::failed(format!("{:?}", error)))
            }
        }
    }

    fn leave(&mut self, params: widget_capnp::market::LeaveParams, mut results: widget_capnp::market::LeaveResults) -> Promise<(), capnp::Error> {
        let id = pry!(params.get()).get_id().unwrap();
        info!("leave requested by account {}", id);
        match self.remove_account(id) {
            Ok(account) => {
                let mut results = results.get();
                let mut builder = results.reborrow().init_account(account.len() as u32);
                account.iter().enumerate().for_each(|(i, (w, c))| {
                    builder.reborrow().get(i as u32).set_widget(w);
                    builder.reborrow().get(i as u32).set_count(*c as i32);
                });
                Promise::ok(())
            }
            Err(error) => {
                error!("unable to get remove account {}", id);
                error!("{:?}", error);
                Promise::err(capnp::Error::failed(format!("{:?}", error)))
            }
        }
    }
}

pub async fn run<M: 'static + Market>(addr: SocketAddr, market: M) -> Result<(), Box<dyn std::error::Error>> {
    LocalSet::new()
        .run_until(async move {
            let listener = TcpListener::bind(&addr).await?;
            let widget_client: widget_capnp::market::Client = capnp_rpc::new_client(market);

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
