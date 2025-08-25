use std::path::PathBuf;

use glob::glob;

fn main() {
    let mut build = cc::Build::new();
    build.include("flanterm/src");
    build.flags(
        "-g -ffreestanding -fno-stack-protector -fno-stack-check -fno-lto -m32 -mno-mmx -mno-sse -mno-sse2 -mno-red-zone -mno-avx -mno-80387".split_ascii_whitespace()
    );
    for entry in glob("flanterm/**/*.c").unwrap() {
        let entry = entry.unwrap();
        let c_path = entry.to_str().unwrap();

        build.file(c_path);

        println!("cargo:rerun-if-changed={c_path}");
    }

    println!("cargo:rustc-link-lib=static=flanterm");

    build.compile("flanterm");

    let mut string = String::new();
    string += "#define FLANTERM_IN_FLANTERM";

    for entry in glob("flanterm/**/*.h").unwrap() {
        let entry = entry.unwrap();
        let h_path = entry.to_str().unwrap();

        string += "#include <";
        string += h_path.strip_prefix("flanterm/src/").unwrap();
        string += ">\n";

        println!("cargo:rerun-if-changed={h_path}");
    }

    std::fs::write("src/wrapper.h", string.into_bytes()).unwrap();

    bindgen::builder()
        .use_core()
        .clang_arg("-I")
        .clang_arg("flanterm/src")
        .clang_arg(format!("--target={}", std::env::var("HOST").unwrap()))
        .derive_debug(true)
        .wrap_unsafe_ops(true)
        .generate_cstr(true)
        .layout_tests(false)
        .header("src/wrapper.h")
        .generate()
        .unwrap()
        .write_to_file(PathBuf::from_iter([
            std::env::var("OUT_DIR").unwrap(),
            "flanterm_bindings.rs".to_string(),
        ]))
        .unwrap();
}
