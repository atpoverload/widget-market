use std::collections::HashMap;

#[derive(Debug)]
pub enum ValidationError {
    AccountError(String),
    MarketError(String),
    TradeError(String),
}

pub trait Market {
    // market viewing
    fn get_market(&self) -> Result<&HashMap<String, i32>, ValidationError>;
    fn get_account(&self, id: &str) -> Result<&HashMap<String, i32>, ValidationError>;
    // account modification
    fn create_account(&mut self) -> Result<String, ValidationError>;
    fn add_account(&mut self, account: HashMap<String, i32>) -> Result<String, ValidationError>;
    fn remove_account(&mut self, id: &str) -> Result<HashMap<String, i32>, ValidationError>;
    // market modification
    fn submit_trade(&mut self, id: &str, buy: &str, sell: &str) -> Result<(), ValidationError>;
}
