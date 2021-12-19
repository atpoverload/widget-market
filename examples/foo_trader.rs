use std::collections::HashMap;
use std::fs;
use std::net::ToSocketAddrs;
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec::Vec;

use clap::{App, Arg};
use log::{error, info};
use serde_json;

use widget_market::client;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = App::new("foo-trader")
        .author("atpoverload")
        .version("0.1.0")
        .about("an simple trader that makes all the trades described in a json file")
        .arg(Arg::with_name("address")
            .short("a")
            .long("address")
            .takes_value(true)
            .required(true)
            .help("address of the server"))
        .arg(Arg::with_name("orders")
            .long("orders")
            .takes_value(true)
            .required(true)
            .help("path to json file of orders"))
        .arg(Arg::with_name("account")
            .long("account")
            .takes_value(true)
            .help("path to json file of an account"))
        .arg(Arg::with_name("output")
            .long("output")
            .takes_value(true)
            .help("path to write the account data as a json"))
        .get_matches();
    let account = match args.value_of("account") {
        Some(path) => {
            let account: HashMap<String, i32> = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
            info!("creating foo trader with account:");
            account.iter().for_each(|(k, v)| {info!(" - {}: {}", k, v);});
            Some(account)
        }
        _ => None
    };
    let addr = args
        .value_of("address")
        .unwrap()
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");
    let order: Vec<(String, String)> = serde_json::from_str(
        &fs::read_to_string(args.value_of("orders").unwrap()).unwrap()).unwrap();

    env_logger::builder().filter(None, log::LevelFilter::Info).init();
    tokio::task::LocalSet::new()
        .run_until(async move {
            // create the rpc client
            let service = client::WidgetMarketClient::new(&addr).await.unwrap();
            let id = match account {
                Some(account) => service.join_with_account(account).await,
                None => service.join().await
            };
            info!("joined server at {} with {}", addr, id);

            info!("proposing {} trades", order.len());
            let mut trades = Vec::new();
            for (i, (buy, sell)) in order.iter().enumerate() {
                if let Ok(_) = service.trade(&id, &buy, &sell).await {
                    trades.push((i, buy, sell));
                }
            }
            info!("submitted {} trades", trades.len());

            let account = service.leave(&id).await;
            let path = match args.value_of("output") {
                Some(path) => path.to_string(),
                _ => format!("{}_{}.json", id, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()),
            };
            info!("writing account details to {}", path);
            if let Err(error) = fs::write(path, serde_json::to_string(&account).unwrap()) {
                error!("an error occurred while writing the account: {}", error);
            }
            Ok(())
        })
        .await
}
