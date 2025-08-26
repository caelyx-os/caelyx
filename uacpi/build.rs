use std::path::PathBuf;

use glob::glob;

fn main() {
    let mut build = cc::Build::new();
    build.include("uacpi/include");
    build.flags(
        "-g -ffreestanding -fno-stack-protector -fno-stack-check -fno-lto -m32 -mno-mmx -mno-sse -mno-sse2 -mno-red-zone -mno-avx -mno-80387".split_ascii_whitespace()
    );

    for entry in glob("uacpi/source/**/*.c").unwrap() {
        let entry = entry.unwrap();
        let c_path = entry.to_str().unwrap();

        build.file(c_path);

        println!("cargo:rerun-if-changed={c_path}");
    }

    println!("cargo:rustc-link-lib=static=uacpi");
    build.compile("uacpi");

    let mut string = String::new();

    for entry in glob("uacpi/include/**/*.h").unwrap() {
        let entry = entry.unwrap();
        let h_path = entry.to_str().unwrap();
        if h_path.contains("internal") {
            continue;
        }

        string += "#include <";
        string += h_path.strip_prefix("uacpi/include/").unwrap();
        string += ">\n";

        println!("cargo:rerun-if-changed={h_path}");
    }

    std::fs::write("src/wrapper.h", string.into_bytes()).unwrap();

    bindgen::builder()
        .use_core()
        .clang_arg("-I")
        .clang_arg("uacpi/include")
        .clang_arg(format!("--target={}", std::env::var("HOST").unwrap()))
        .derive_debug(true)
        .derive_default(true)
        .wrap_unsafe_ops(true)
        .generate_cstr(true)
        .layout_tests(false)
        .header("src/wrapper.h")
        .generate()
        .unwrap()
        .write_to_file(PathBuf::from_iter([
            std::env::var("OUT_DIR").unwrap(),
            "uacpi_bindings.rs".to_string(),
        ]))
        .unwrap();
}
