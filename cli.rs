use std::net::SocketAddr;
use std::fs;

use clap::{App, Arg, ArgMatches};
use log::info;

use crate::args;
use crate::client;

pub fn cli_command() -> App<'static, 'static> {
    App::new("client")
        .about("runs a client to interact with a market server")
        .subcommand(App::new("join")
            .about("joins a market")
            .after_help("requests to join a market server, returning an account id")
            .arg(args::addr_arg()))
        .subcommand(App::new("check")
            .about("checks an account's market view")
            .after_help("checks an account's view of the market, returning a snapshot")
            .arg(args::id_arg())
            .arg(args::addr_arg()))
        .subcommand(App::new("trade")
            .arg(Arg::with_name("buy")
                .required(true))
            .arg(Arg::with_name("sell")
                .required(true))
            .about("requests a widget trade")
            .after_help("requests a trade be made, returning if it was accepted")
            .arg(args::id_arg())
            .arg(args::addr_arg()))
        .subcommand(App::new("order")
            .arg(Arg::with_name("order")
                .required(true))
            .about("requests a widget trade order")
            .after_help("requests a order of trades be made, returning if each was accepted")
            .arg(args::id_arg())
            .arg(args::addr_arg()))
        .subcommand(App::new("leave")
            .about("leaves a market")
            .after_help("leaves a market, returning a score")
            .arg(args::id_arg())
            .arg(args::addr_arg()))
}

fn show_trade(id: &str, buy: &str, sell: &str, result: bool) {
    info!(
        "account {} wanted to buy {} for {}; trade was {}",
        id,
        buy,
        sell,
        if result {"accepted"} else {"rejected"}
    );
}

pub async fn main(command: &str, addr: SocketAddr, args: &ArgMatches<'_>) -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder().filter(None, log::LevelFilter::Info).init();
    tokio::task::LocalSet::new().run_until(async move {
        // create the rpc client
        let service = client::WidgetMarketClient::new(&addr).await.unwrap();

        // parse the command
        let id = args.value_of("id").unwrap_or("");
        match command {
            "join" => {
                let id = service.join().await;
                info!("joined market at {} with id {}", addr, id);
                println!("{}", id);
            }
            "check" => {
                let snapshot = service.check(id).await;
                info!("market: {:?}", snapshot.market);
                info!("{} account: {:?}", args.value_of("id").unwrap(), snapshot.account);
            },
            "trade" => {
                let buy = args.value_of("buy").unwrap();
                let sell = args.value_of("sell").unwrap();
                let result = service.trade(id, buy, sell).await;
                show_trade(id, buy, sell, result);
                println!("{}", result);
            },
            "order" => {
                for order in fs::read_to_string(args.value_of("order").unwrap())?.lines() {
                    let widgets: Vec<&str> = order.split(',').collect();
                    let buy = widgets.get(0).unwrap();
                    let sell = widgets.get(1).unwrap();
                    let result = service.trade(id, buy, sell).await;
                    show_trade(id, buy, sell, result);
                    println!("{}", result)
                }
            }
            "leave" => {
                let score = service.leave(id).await;
                info!("account {} left market at {}; scored {} points", id, addr, score);
                println!("{}", score);
            }
            // throw here
            _ => (),
        };

        Ok(())
    }).await
}
