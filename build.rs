extern crate capnpc;

fn main() {
    ::capnpc::CompilerCommand::new().src_prefix("src").file("schema/widget.capnp").run().unwrap()
}
