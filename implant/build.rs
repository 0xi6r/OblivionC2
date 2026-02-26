fn main() {
    // Compile C2 protocol
    prost_build::compile_protos(&["src/protocol/c2.proto"], &["src/"])
        .expect("Failed to compile C2 protocol");
    
    // Compile Operator protocol
    tonic_build::compile_protos("proto/operator.proto")
        .expect("Failed to compile operator protocol");
}