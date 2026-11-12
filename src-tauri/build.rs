fn main() {
    // Picks the update feed (see `update_target`).
    println!("cargo:rerun-if-env-changed=SUB20_BUNDLED");
    tauri_build::build()
}
