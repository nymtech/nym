extern crate bindgen;

use cfg_if;
use std::{env, path::PathBuf, process::Command};

use bindgen::CargoCallbacks;

fn main() {
    // This is the directory where the `c` library is located.
    let libdir_path = PathBuf::from("cpucycles/")
        // Canonicalize the path as `rustc-link-search` requires an absolute
        // path.
        .canonicalize()
        .expect("cannot canonicalize path");

    // This is the path to the `c` headers file.
    let headers_path = libdir_path.join("cpucycles.h");
    let headers_path_str = headers_path.to_str().expect("Path is not a valid string");

    // This is the path to the intermediate object file for our library.
    let obj_path = libdir_path.join("cpucycles.o");
    // This is the path to the static library file.
    let lib_path = libdir_path.join("libcpucycles.a");
    let lib_path_str = lib_path.to_str().expect("Path is not a valid string");

    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={}", lib_path_str);

    // Tell cargo to tell rustc to link our `hello` library. Cargo will
    // automatically know it must look for a `libhello.a` file.
    println!("cargo:rustc-link-lib=cpucycles");

    // Tell cargo to invalidate the built crate whenever the header changes.
    println!("cargo:rerun-if-changed={}", headers_path_str);

    let src_path: String;

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "x86")] {
            src_path = "x86-tscasm.c".to_string()
        }
        else if #[cfg(target_arch = "x86_64")] {
            src_path = "amd64-tscasm.c".to_string()
        } else if #[cfg(target_arch = "mips")] {
            src_path = "mips64-cc.c".to_string()
        } else if #[cfg(target_arch = "powerpc")] {
            src_path = "ppc32-mftb.c".to_string()
        } else if #[cfg(target_arch = "powerpc64")] {
            src_path = "ppc64-mftb.c".to_string()
        } else if #[cfg(all(target_arch = "arm", target_pointer_width = "64"))] {
            src_path = "arm64-pmc.c".to_string()
        } else {
            panic!("Unsupported architecture ({:?})!", env::var("CARGO_CFG_TARGET_ARCH"), )
        }
    };

    // Run `clang` to compile the `hello.c` file into a `hello.o` object file.
    // Unwrap if it is not possible to spawn the process.
    let mut compile_o_command = Command::new("clang");
    let compile_o_command = compile_o_command
        .arg("-c")
        .arg("-o")
        .arg(&obj_path)
        .arg(libdir_path.join(&src_path));

    println!("Running: {:?}", compile_o_command);

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

    // Run `ar` to generate the `libhello.a` file from the `hello.o` file.
    // Unwrap if it is not possible to spawn the process.
    if !std::process::Command::new("ar")
        .arg("rcs")
        .arg(&lib_path)
        .arg(obj_path)
        .output()
        .expect("could not spawn `ar`")
        .status
        .success()
    {
        // Panic if the command was not successful.
        panic!("could not emit library file");
    }

    println!("cargo:rustc-link-arg={}", lib_path_str);

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(headers_path_str)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    bindings
        .write_to_file("./src/bindings.rs")
        .expect("Couldn't write bindings!");
}
