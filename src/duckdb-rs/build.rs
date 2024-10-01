use std::path::PathBuf;
use tonic_build::configure;

fn main() {
    let proto_path: PathBuf = "proto/matrix_cache.proto".into();
    let proto_dir = proto_path
        .parent()
        .expect("proto file should reside in a directory");
    configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&["proto/matrix_cache.proto"], &[proto_dir])
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
}
