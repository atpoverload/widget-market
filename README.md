# `widget-market`

a simulation framework for building and interacting with market servers written in [`rust`](https://www.rust-lang.org/tools/install).

## requirements

`widget-market` is written in [`rust`](https://www.rust-lang.org/tools/install) and uses [`capnp`](https://capnproto.org) for serialization and communication. to use `widget-market`, you'll need to install [`capnpc`](https://capnproto.org/install.html#installation-unix). you should be able to install both of these through a package manager like [`chocolatey`](https://chocolatey.org/) or [`apt`](https://packages.debian.org/sid/capnproto).

## cli

`widget-trader` has a cli tool that is used to talk to market servers. any command will require the address of the server you want to talk to:

```bash
cargo run -- --address=$server_address join
```

joining a market will output an id, which you can use to do other market operations:

```bash
# joins a market, stores the market id, and then checks the market
id=$(cargo run -- --address=$server_address join)
cargo run -- --address=$server_address check --id=$id

# uses a function to pick two widgets to trade
widgets_to_trade=pick_widgets($(cargo run -- --address=$server_address check --id=$id), 2)
cargo run -- --address=$server_address trade --id=$id $widgets

# writes account to "unixtime_market.json"
output"${output_dir}/$(date +%s)_${id}.json"
cargo run -- --address=$server_address leave --id=$id --output=$output
```

the [client](src/client.rs) is also publicly provided so it can be used in a custom application.

## implementing a market

the tougher part is implementing a market. the market [schema](schema/widget.capnp) is very simple, which means that the actual implementation is not. `widget-market` provides a lightweight framework to build modular market servers. by implementing [`Market`](market.rs), a new server can be constructed quickly. an example market implementation is provided at [foo_market](src/foo_market.rs).
