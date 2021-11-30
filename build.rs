extern crate capnpc;

fn main() {
    ::capnpc::CompilerCommand::new().file("widget.capnp").run().unwrap();
}
