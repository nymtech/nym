fn main() {
    uniffi::generate_scaffolding("src/bindings.udl").unwrap();
}
