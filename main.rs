pub mod widget_capnp {
  include!(concat!(env!("OUT_DIR"), "/widget_capnp.rs"));
}

use std::net::ToSocketAddrs;

use clap::App;

pub mod args;
pub mod cli;
pub mod client;
pub mod server;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = App::new("widget-market")
        .author("atpoverload")
        .version("0.1.0")
        .about("a cli tool for running widget trading markets")
        .after_help("a command line tool that can run customizable market servers and make transactions with running servers")
        .subcommand(server::server_command())
        .subcommand(cli::cli_command())
        .get_matches();

    let (subcommand, args) = args.subcommand();
    let args = args.unwrap();

    match subcommand {
        "server" => server::main(
            args.value_of("address")
                .unwrap()
                .to_socket_addrs()?
                .next()
                .expect("could not parse address"),
            args
        ).await,
        _ => {
            let (subcommand, args) = args.subcommand();
            let args = args.unwrap();
            cli::main(
                subcommand,
                args.value_of("address")
                    .unwrap()
                    .to_socket_addrs()?
                    .next()
                    .expect("could not parse address"),
                args
            ).await
        },
    }
}
