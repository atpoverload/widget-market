use std::fs;
use std::net::ToSocketAddrs;

use clap::{App, Arg};
use log::{error, info};
use serde_json;

use widget_market::client;

pub fn id_arg() -> Arg<'static, 'static> {
    Arg::with_name("id")
        .long("id")
        .takes_value(true)
        .required(true)
        .help("account id for the market")
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = App::new("widget-market cli")
        .author("atpoverload")
        .version("0.1.0")
        .about("runs a client to interact with a market")
        .subcommand(App::new("join")
            .about("joins a market")
            .arg(Arg::with_name("account")
                .long("account")
                .takes_value(true)
                .help("path to an account as a json"))
            .after_help("requests to join a market server, returning an account id"))
        .subcommand(App::new("check")
            .about("checks an account's market view")
            .after_help("checks an account's view of the market, returning a snapshot")
            .arg(id_arg()))
        .subcommand(App::new("trade")
            .arg(Arg::with_name("buy").required(true))
            .arg(Arg::with_name("sell").required(true))
            .about("requests a widget trade")
            .after_help("requests a trade be made, returning if it was accepted")
            .arg(id_arg()))
        .subcommand(App::new("leave")
            .arg(Arg::with_name("output")
                .long("output")
                .takes_value(true)
                .help("path to write the account data"))
            .about("leaves a market")
            .after_help("leaves a market, returning a score")
            .arg(id_arg()))
        .arg(Arg::with_name("address")
            .short("a")
            .long("address")
            .takes_value(true)
            .required(true)
            .help("address of the server"))
        .get_matches();

        let addr = args
            .value_of("address")
            .unwrap()
            .to_socket_addrs()?
            .next()
            .expect("could not parse address");
        let (command, args) = args.subcommand();
        let args = args.unwrap();

        env_logger::builder().filter(None, log::LevelFilter::Info).init();
        tokio::task::LocalSet::new()
            .run_until(async move {
                // create the rpc client
                let service = client::WidgetMarketClient::new(&addr).await.unwrap();

                // parse the command
                match command {
                    "join" => {
                        let id = match args.value_of("account") {
                            Some(path) => service.join_with_account(
                                serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()).await,
                            _ => service.join().await,
                        };
                        info!("joined market at {} with id {}", addr, id);
                        println!("{}", id);
                    }
                    "check" => {
                        let id = args.value_of("id").expect("no id was provided");
                        let snapshot = service.check(id).await;
                        info!("market: {:?}", snapshot.1);
                        info!("{} account: {:?}", id, snapshot.0);
                    }
                    "trade" => {
                        let id = args.value_of("id").expect("no id was provided");
                        let buy = args.value_of("buy").unwrap();
                        let sell = args.value_of("sell").unwrap();
                        let result = service.trade(id, buy, sell).await;
                        info!("{} proposed {} -> {}", id, buy, sell);
                        info!("{}", match result {Ok(_) => "submitted".to_string(), Err(e) => e});
                    }
                    "leave" => {
                        let id = args.value_of("id").expect("no id was provided");
                        let account = service.leave(id).await;
                        let path = match args.value_of("output") {
                            Some(path) => path.to_string(),
                            _ => format!("{}.json", id),
                        };
                        info!("wrote account details to {}", path);
                        if let Err(error) = fs::write(path, serde_json::to_string(&account).unwrap()) {
                            error!("an error occurred while writing the account: {}", error);
                        }
                    }
                    // throw here
                    _ => (),
                };

                Ok(())
            })
            .await
}
