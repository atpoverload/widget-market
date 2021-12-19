// implementation for a very simple market that is backed by hash maps
//
// the market has the following properties:
//  - the market's can be created with either a hash map of str->int or a json map of ints
//  - new accounts are given 1 of each widget
//  - added accounts start with their provided widgets
//  - the market reports the exact contents for both itself and accounts
//  - the market's will not allow trades of new widgets
//  - the market's will not allow trades of identical widgets
//  - the market's will not allow trades of with insufficient widgets
//  - all widgets are worth the same amount
//  - trades are done immediately
//  - accounts are removed and returned to the user when leaving

use std::net::ToSocketAddrs;
use std::collections::HashMap;
use std::fs::read_to_string;

use clap::{App, Arg};
use log::{debug, info};
use rand::{distributions::Alphanumeric, Rng};
use serde_json;

use widget_market::market::{Market, ValidationError};
use widget_market::single_market;

fn new_id(size: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .map(char::from)
        .collect()
}

#[derive(Clone, Debug)]
struct FooMarket {
    market: HashMap<String, i32>,
    accounts: HashMap<String, HashMap<String, i32>>,
}

impl FooMarket {
    fn from_map(widgets: HashMap<String, i32>) -> FooMarket {
        FooMarket {
            market: widgets,
            accounts: HashMap::new(),
        }
    }

    fn from_json(path: &str) -> FooMarket {
        FooMarket::from_map(serde_json::from_str(&read_to_string(path).unwrap()).unwrap())
    }

    fn has_account(&self, id: &str) -> Result<(), ValidationError> {
        if self.accounts.contains_key(id) {
            Ok(())
        } else {
            Err(ValidationError::AccountError(format!("account {} does not exist",id)))
        }
    }

    fn new_account(&self) -> HashMap<String, i32> {
        self.market.keys().map(|widget| (widget.to_owned(), 1)).collect()
    }

    fn get_costs(&self, _: &str, _: &str) -> (i32, i32) {
        (1, 1)
    }

    fn make_trade(&mut self, id: &str, buy: &str, sell: &str, buy_cost: i32, sell_cost: i32) -> Result<(), ValidationError> {
        if self.market.get(buy).unwrap() < &buy_cost {
            Err(ValidationError::TradeError(format!("not enough {} in market", buy)))
        } else if self.accounts.get(id).unwrap().get(sell).unwrap() < &sell_cost {
            Err(ValidationError::TradeError(format!("not enough {} in account {}", sell, id)))
        } else {
            self.market
                .entry(buy.to_string())
                .and_modify(|widgets| *widgets -= buy_cost);
            self.market
                .entry(sell.to_string())
                .and_modify(|widgets| *widgets += sell_cost);
            self.accounts.entry(id.to_string()).and_modify(|account| {
                account
                    .entry(buy.to_string())
                    .and_modify(|widgets| *widgets += buy_cost);
                account
                    .entry(sell.to_string())
                    .and_modify(|widgets| *widgets -= sell_cost);
            });
            Ok(())
        }
    }
}

impl Market for FooMarket {
    fn get_market(&self) -> Result<&HashMap<String, i32>, ValidationError> {
        Ok(&self.market)
    }

    fn get_account(&self, id: &str) -> Result<&HashMap<String, i32>, ValidationError> {
        match self.has_account(id) {
            Ok(_) => Ok(self.accounts.get(id).unwrap()),
            Err(error) => Err(error),
        }
    }

    fn create_account(&mut self) -> Result<String, ValidationError> {
        self.add_account(self.new_account())
    }

    fn add_account(&mut self, account: HashMap<String, i32>) -> Result<String, ValidationError> {
        let id: String = new_id(10);
        if self.accounts.contains_key(&id) {
            Err(ValidationError::MarketError(format!("account {} already exists", id)))
        } else {
            let mut account: HashMap<String, i32> = account.iter()
                .filter(|(k, _)| self.market.contains_key(&k.to_string()))
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect();
            self.market.keys().for_each(|w| {account.entry(w.to_string()).or_insert(0);});
            self.accounts.insert(id.to_owned(), account);
            Ok(id)
        }
    }

    fn remove_account(&mut self, id: &str) -> Result<HashMap<String, i32>, ValidationError> {
        match self.has_account(id) {
            Ok(_) => Ok(self.accounts.remove(id).unwrap()),
            Err(error) => Err(error),
        }
    }

    fn submit_trade(&mut self, id: &str, buy: &str, sell: &str) -> Result<(), ValidationError> {
        match self.has_account(id) {
            Ok(_) => {
                if buy == sell {
                    Err(ValidationError::TradeError(format!("both widgets are {}", buy)))
                } else if !self.market.contains_key(buy) {
                    Err(ValidationError::TradeError(format!("{} is not in market", buy)))
                } else if !self.market.contains_key(sell) {
                    Err(ValidationError::TradeError(format!("{} is not in market", sell)))
                } else {
                    let (buy_cost, sell_cost) = self.get_costs(buy, sell);
                    self.make_trade(id, buy, sell, buy_cost, sell_cost)
                }
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_market() -> HashMap<String, i32> {
        ["foo", "bar", "baz"]
            .iter()
            .map(|&widget| (widget.to_string(), 10))
            .collect()
    }

    fn new_account() -> HashMap<String, i32> {
        ["foo", "bar", "baz"]
            .iter()
            .map(|&widget| (widget.to_string(), 1))
            .collect()
    }

    fn used_account() -> HashMap<String, i32> {
        let mut account = HashMap::new();
        account.insert("foo".to_string(), 2);
        account.insert("bar".to_string(), 0);
        account.insert("baz".to_string(), 1);
        account
    }

    fn new_account_2() -> HashMap<String, i32> {
        ["foo", "bar", "baz"]
            .iter()
            .map(|&widget| (widget.to_string(), 2))
            .collect()
    }

    // TODO: think about these test cases some more; i don't think we **really** exhausted this
    #[test]
    fn test_market_impl() {
        let mut market = FooMarket::from_map(new_market());

        // make sure the market matches
        assert_eq!(market.get_market().unwrap(), &new_market());

        // try to get a fake account
        let id = "fake id";
        market.get_account(&id).expect_err("shouldn't have been an account!");

        // try to get a real, new account
        let id = market.create_account().unwrap();
        assert_eq!(market.get_account(&id).unwrap(), &new_account());

        // try to trade non-existent widget
        market.submit_trade(&id, "foo", "bang").expect_err("shouldn't be able to trade baz");

        // try to trade two foos
        market.submit_trade(&id, "foo", "foo").expect_err("shouldn't be able to trade two foos");

        // try to trade
        assert_eq!(market.submit_trade(&id, "foo", "bar").unwrap(), ());
        assert_eq!(market.get_account(&id).unwrap(), &used_account());

        // try to trade without resources left
        market.submit_trade(&id, "baz", "bar").expect_err("shouldn't be any bar left");
        market.submit_trade(&id, "foo", "bar").expect_err("shouldn't be any bar left");

        // try to trade back
        assert_eq!(market.submit_trade(&id, "bar", "foo").unwrap(), ());
        assert_eq!(market.get_account(&id).unwrap(), &new_account());
        assert_eq!(market.remove_account(&id).unwrap(), new_account());

        // try to add an account with widgets
        let id = market.add_account(new_account_2()).unwrap();
        assert_eq!(market.get_account(&id).unwrap(), &new_account_2());
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = App::new("foo-market")
        .author("atpoverload")
        .version("0.1.0")
        .about("an example of an implemented market to trade foo widgets")
        .arg(Arg::with_name("address")
            .short("a")
            .long("address")
            .takes_value(true)
            .required(true)
            .help("address of the server"))
        .arg(Arg::with_name("market")
            .long("market")
            .takes_value(true)
            .help("path to an market as a json"))
        .get_matches();

    env_logger::builder().filter(None, log::LevelFilter::Debug).init();

    let market = match args.value_of("market") {
        Some(path) => FooMarket::from_json(path),
        _ => {
            debug!("no market provided; creating a new market");
            FooMarket::from_map(serde_json::from_str("{\"foo\": 1000, \"bar\": 1000}").unwrap())
        }
    };
    let addr = args
        .value_of("address")
        .unwrap()
        .to_socket_addrs()
        .unwrap()
        .next()
        .expect("could not parse address");
    info!("starting foo market server at {} with contents:", addr);
    market.get_market().unwrap().iter().for_each(|(k, v)| {info!(" - {}: {}", k, v);});

    single_market::run(addr, market).await
}
