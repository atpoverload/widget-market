use std::collections::HashMap;
use std::fs::read_to_string;
use std::net::SocketAddr;
use std::vec::Vec;

use capnp;
use capnp::capability::Promise;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, pry, RpcSystem};
use clap::{App, Arg, ArgMatches};
use futures::{AsyncReadExt, FutureExt};
use log::{error, debug, info};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use tokio::task::{LocalSet, spawn_local};
use tokio::net::TcpListener;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::args;
use crate::widget_capnp::widget_market;

#[derive(Debug)]
enum ValidationError {
    AccountDoesntExist(String),
    IdenticalWidgets(String),
    NonTradeableWidget(String),
    InsufficientAmount(String),
}

pub fn server_command() -> App<'static, 'static> {
    App::new("server")
        .about("starts a market server from a provided config")
        .arg(Arg::with_name("market")
            .long("market")
            .takes_value(true)
            .help("path to a market configuration"))
        .arg(args::addr_arg())
}

fn format_transaction(widget: &str, old_count: &i32, new_count: &i32) -> String {
    let diff = new_count - old_count;
    format!(
            " - {: >3}: {: >3} -> {: >3} {}",
            widget,
            old_count,
            new_count,
            if diff != 0 {format!("({: >3})", format!("{:+}", diff))} else {"".to_string()}
    )
}

// this is not generalized at all
// this might need to be injected as a trait
#[derive(Debug, Serialize, Deserialize)]
struct WidgetMarketImpl {
    market: HashMap<String, i32>,
    accounts: HashMap<String, HashMap<String, i32>>,
}

trait Market {
    fn create_account(&self) -> String;
    fn get_market(&self) -> HashMap<String, i32>;
    fn get_account(&self, id: &str) -> HashMap<String, i32>;
    fn remove_account(&self, id: &str);
}

fn new_account(count: i32) -> HashMap<String, i32> {
    ["foo", "bar", "baz"].iter().map(|&widget| (widget.to_string(), count)).collect()
}

impl WidgetMarketImpl {
    fn validate_widget(&self, id: &str, buy: &str, sell: &str) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        if !self.accounts.contains_key(id) {
            errors.push(ValidationError::AccountDoesntExist(id.to_string()));
        }

        if buy == sell {
            errors.push(ValidationError::IdenticalWidgets(buy.to_string()));
        }

        for widget in vec![buy, sell] {
            if !self.market.contains_key(widget) {
                errors.push(ValidationError::NonTradeableWidget(widget.to_string()));
            }
        }

        if !errors.is_empty() {
            Err(errors)
        } else {
            Ok(())
        }
    }

    fn validate_transaction(&self, id: &str, buy: &str, buy_amount: i32, sell: &str, sell_amount: i32) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        if let Some(amount) = self.market.get(buy) {
            if amount < &buy_amount {
                errors.push(ValidationError::InsufficientAmount("market".to_string()))
            }
        }

        if let Some(amount) = self.accounts.get(id).unwrap().get(sell) {
            if amount < &sell_amount {
                errors.push(ValidationError::InsufficientAmount(id.to_string()))
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        } else {
            Ok(())
        }
    }
}

fn add_account(items: &HashMap<String, i32>, account: widget_market::account::Builder) {
    let mut widgets = account.init_widgets(items.len() as u32);
    items.iter().enumerate().for_each(|(i, (w, c))| {
        widgets.reborrow().get(i as u32).set_widget(w);
        widgets.reborrow().get(i as u32).set_count(*c as i32);
    });
}

impl widget_market::Server for WidgetMarketImpl {
    fn join(&mut self, _: widget_market::JoinParams, mut results: widget_market::JoinResults) -> Promise<(), capnp::Error> {
        info!("join requested");
        // this should be a 'self' method
        let id: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(10)
            .map(char::from)
            .collect();
        self.accounts.entry(id.to_owned()).or_insert_with(|| {
            info!("creating new account {}", id);
            // this should be a 'self' method
            new_account(1)
        });

        results.get().set_id(&id);
        Promise::ok(())
    }

    fn check(&mut self, params: widget_market::CheckParams, mut results: widget_market::CheckResults) -> Promise<(), capnp::Error> {
        let id = pry!(params.get()).get_id().unwrap();
        info!("check requested by account {}", id);
        // this should be a 'self' method
        if !self.accounts.contains_key(id) {
            return Promise::err(capnp::Error::failed(
                format!("{:?}", ValidationError::AccountDoesntExist(id.to_string()))
            ));
        }

        let mut market = results.get().init_market();
        add_account(&self.market, market.reborrow().init_market());
        add_account(self.accounts.get(id).unwrap(), market.reborrow().init_account());
        info!("sending snapshot to account {}", id);

        Promise::ok(())
    }

    fn trade(&mut self, params: widget_market::TradeParams, _: widget_market::TradeResults) -> Promise<(), capnp::Error> {
        // grab the params
        let transaction = pry!(params.get()).get_transaction().unwrap();
        let id = transaction.get_id().unwrap();
        let buy = transaction.get_buy().unwrap();
        let sell = transaction.get_sell().unwrap();
        info!("trade requested by account {}", id);

        // validate the params
        // should this be part of a validator?
        if let Err(errors) = self.validate_widget(id, buy, sell) {
            info!("trade rejected");
            errors.iter().for_each(|e| error!(" - {:?}", e));
            return Promise::err(capnp::Error::failed(errors
                .iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<String>>()
                .join(",")));
        };


        // we want to generalize this
        // this should be a 'self' method
        let buy_amount = 1;
        let sell_amount = 1;

        info!("account {} wants to buy {}({}) for {}({})", id, buy, buy_amount, sell, sell_amount);

        // validate the trade
        if let Err(errors) = self.validate_transaction(id, buy, buy_amount, sell, sell_amount) {
            info!("trade rejected");
            errors.iter().for_each(|e| error!(" - {:?}", e));
            return Promise::err(capnp::Error::failed(errors
                .iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<String>>()
                .join(",")));
        };

        // update the market
        // this should be a 'self' method
        let old_market = self.market.clone();
        let old_account = self.accounts.get(id).unwrap().clone();

        self.market.entry(buy.to_string()).and_modify(|c| *c -= buy_amount);
        self.market.entry(sell.to_string()).and_modify(|c| *c += sell_amount);
        self.accounts.entry(id.to_string()).and_modify(|e| {
            e.entry(buy.to_string()).and_modify(|c| *c += buy_amount);
            ()
        });
        self.accounts.entry(id.to_string()).and_modify(|e| {
            e.entry(sell.to_string()).and_modify(|c| *c -= sell_amount);
            ()
        });

        info!("trade accepted");
        debug!("new market:");
        old_market.iter().zip(self.market.iter())
            .map(|((w, o), (_, n))| format_transaction(w, o, n))
            .for_each(|transaction| debug!("{}", transaction));
        debug!("{} account:", id);
        old_account.iter().zip(self.accounts.get(id).unwrap().iter())
            .map(|((w, o), (_, n))| format_transaction(w, o, n))
            .for_each(|transaction| debug!("{}", transaction));

        Promise::ok(())
    }

    fn leave(&mut self, params: widget_market::LeaveParams, mut results: widget_market::LeaveResults) -> Promise<(), capnp::Error> {
        let id = pry!(params.get()).get_id().unwrap();
        info!("leave requested by account {}", id);
        // this should be a 'self' method
        if !self.accounts.contains_key(id) {
            return Promise::err(capnp::Error::failed(
                format!("{:?}", ValidationError::AccountDoesntExist(id.to_string()))
            ));
        }
        // this should be a 'self' method
        results.get().set_score(self.accounts.get(id).unwrap().iter().map(|(_, c)| c).sum());
        self.accounts.remove(id);
        info!("destroyed account {}", id);

        Promise::ok(())
    }
}

pub async fn main(addr: SocketAddr, args: &ArgMatches<'_>) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder().filter(None, log::LevelFilter::Debug).init();
    LocalSet::new().run_until(async move {
        let listener = TcpListener::bind(&addr).await?;
        let config = match args.value_of("market") {
            Some(config) => serde_json::from_str(&read_to_string(config).unwrap()).unwrap(),
            _ => WidgetMarketImpl {
                market: new_account(100),
                accounts: HashMap::new(),
            },
        };
        info!("{:?}", config);
        let widget_client: widget_market::Client = capnp_rpc::new_client(config);

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
            let rpc_system = RpcSystem::new(Box::new(network), Some(widget_client.clone().client));

            spawn_local(Box::pin(rpc_system.map(|_| ())));
        }
    }).await
}
