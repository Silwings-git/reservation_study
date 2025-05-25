fn main() {
    tonic_build::configure()
        .out_dir("src/pb")
        .compile_protos(&["./protos/reservation.proto"], &["protos"])
        .unwrap();
    println!("cargo:rerun-if-changed=protos/reservation.proto");
    println!("cargo:rerun-if-changed=build.rs");
}
