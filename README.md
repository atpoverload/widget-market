# `widget-market`

a toy simulation framework for trading in a market.

## usage

`widget-trader` has a cli tool that is used to run and communicate with servers. you will only need to install `[cargo](https://www.rust-lang.org/tools/install)` to use it. once you've installed rust, open two terminals and navigate them to the directory where this project is.

in the first terminal, run the following to start a server:

```bash
# starts up a new server
cargo run -- server --address 127.0.0.1:4000
```

the server should start quickly. once it does, run the following in the second terminal:

```bash
# starts up a new server
ACCOUNT_ID=$(cargo run -- client join --address 127.0.0.1:4000)
cargo run -- client check --address 127.0.0.1:4000 --id ${ACCOUNT_ID}
```

this should print out a message with widget counts for both an account that was just made and the market itself. from here, you can use the `trade` or `order` command to make trades on the server.
