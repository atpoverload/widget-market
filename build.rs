extern crate capnpc;

fn main() {
    ::capnpc::CompilerCommand::new().src_prefix("schema").file("schema/widget.capnp").run().unwrap()
}
