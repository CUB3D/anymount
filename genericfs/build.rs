use std::io::Result;

fn main() -> Result<()> {
    let mut prost_build = prost_build::Config::new();
    // prost_build.protoc_executable(protobuf_src::protoc());

    prost_build.compile_protos(&["src/chromeos_ota/update_metadata.proto"], &["src/chromeos_ota"])?;
    Ok(())
}