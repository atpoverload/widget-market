@0xa289e6e439fadcd7;

interface WidgetMarket {
  # joins the market, getting an account id
  # TODO(timur): this is insecure because anyone can ping with an id if they
  #  know it and make trades. should there be another security layer?
  # TODO(timur): right now we can only create and destroy accounts; how do we
  #  let people jump between markets
  join @0 () -> (id :Text);

  # checks the current market from the account's perspective
  check @1 (id :Text) -> (market :Market);

  # snapshot of the current market which
  struct Market {
    account @0 :Account;
    market @1 :Account;
  }

  struct Account {
    widgets @0 :List(WidgetCount);
  }

  struct WidgetCount {
    widget @0 :Text;
    count @1 :Int32;
  }

  # requests to trade a widget for another widget
  # TODO(timur): we can make the transactions more general to handle things like
  #  predicates
  trade @2 (transaction :Transaction) -> ();

  struct Transaction {
    id @0 :Text;
    buy @1 :Text;
    sell @2 :Text;
  }

  # TODO(timur): we can return some sort of bundle
  leave @3 (id :Text) -> (score :Int32);
}
