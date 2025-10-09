fn main() {
    if let Ok(bin_name) = std::env::var("CARGO_BIN_NAME") {
        println!("cargo:warning=CARGO_BIN_NAME={}", bin_name);
    } else {
        println!("cargo:warning=CARGO_BIN_NAME not set");
    }

    slint_build::compile("src/bin/slint_demo/main.slint").unwrap();
}
