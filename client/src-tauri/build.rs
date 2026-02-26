fn main() {
    tauri_build::build();
    
    // Compile protobuf
    tonic_build::compile_protos("../../proto/operator.proto")
        .expect("Failed to compile operator protocol");
}