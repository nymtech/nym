use std::{env, path::PathBuf, process::Command};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = PathBuf::from(&out_dir);
    let source_path = PathBuf::from("libcpucycles")
        .canonicalize()
        .expect("cannot canonicalize path");

    cfg_if::cfg_if! {
        if #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "mips", target_arch = "powerpc", target_arch = "powerpc64", target_arch = "arm", target_arch = "aarch64")))] {
            panic!("Unsupported architecture - {}!", env::var("CARGO_CFG_TARGET_ARCH").unwrap(), )
        }
    };

    let mut compile_o_command = Command::new("./configure");
    let compile_o_command = compile_o_command
        .current_dir(&source_path)
        .arg(format!("--prefix={out_dir}"));

    match compile_o_command.output() {
        Ok(output) => {
            if !output.status.success() {
                panic!("{:?}", unsafe {
                    std::str::from_utf8_unchecked(&output.stderr)
                })
            }
        }
        Err(e) => panic!("{e}"),
    }

    let mut compile_o_command = Command::new("make");
    let compile_o_command = compile_o_command.current_dir(&source_path).arg("install");

    match compile_o_command.output() {
        Ok(output) => {
            if !output.status.success() {
                panic!("{:?}", unsafe {
                    std::str::from_utf8_unchecked(&output.stderr)
                })
            }
        }
        Err(e) => panic!("{e}"),
    }

    println!(
        "cargo:rustc-link-search=native={}",
        out_path.join("lib").to_str().unwrap()
    );
    println!("cargo:rustc-link-lib=static=cpucycles");

    let mut compile_o_command = Command::new("make");
    let compile_o_command = compile_o_command.current_dir(source_path).arg("clean");

    match compile_o_command.output() {
        Ok(output) => {
            if !output.status.success() {
                panic!("{:?}", unsafe {
                    std::str::from_utf8_unchecked(&output.stderr)
                })
            }
        }
        Err(e) => panic!("{e}"),
    }
}
