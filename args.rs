use clap::Arg;

pub fn id_arg() -> Arg<'static, 'static> {
    Arg::with_name("id")
        .long("id")
        .takes_value(true)
        .required(true)
        .help("account id for the server")
}

pub fn addr_arg() -> Arg<'static, 'static> {
    Arg::with_name("address")
        .short("a")
        .long("address")
        .takes_value(true)
        .required(true)
        .help("address of the server")
}
