fn main() {
    tonic_prost_build::configure()
        .build_server(true)
        .compile_protos(&["proto/ext_authz.proto"], &["proto"])
        .unwrap();
}
