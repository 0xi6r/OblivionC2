fn main() {
    prost_build::compile_protos(&["src/protocol/c2.proto"], &["src/"])
        .expect("Failed to compile protobuf");
}