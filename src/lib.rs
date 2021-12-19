pub mod client;
pub mod market;
pub mod server;

pub mod widget_capnp {
    include!(concat!(env!("OUT_DIR"), "/schema/widget_capnp.rs"));
}
