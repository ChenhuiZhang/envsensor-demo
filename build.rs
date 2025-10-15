use std::env;

fn main() {
    if let Ok(bin_name) = env::var("CARGO_BIN_NAME") {
        println!("cargo:warning=CARGO_BIN_NAME={}", bin_name);
    } else {
        println!("cargo:warning=CARGO_BIN_NAME not set");
    }

    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        winres::WindowsResource::new()
            .set_icon("asset/icon.ico")
            .compile()
            .unwrap();
    }
    slint_build::compile("src/bin/slint_demo/main.slint").unwrap();
}
