pub mod client;
pub mod market;
pub mod single_market;

pub mod widget_capnp {
    include!(concat!(env!("OUT_DIR"), "/schema/widget_capnp.rs"));
}
