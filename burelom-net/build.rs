use std::io::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=proto/packet.proto");
    println!("cargo:rerun-if-changed=proto/");

    prost_build::compile_protos(&["proto/packet.proto"], &["proto/"])?;
    Ok(())
}