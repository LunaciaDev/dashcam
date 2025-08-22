use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/ffmpeg_encoder/encoder.c");

    let mut cfg = cc::Build::new();
    cfg.file("src/ffmpeg_encoder/encoder.c");

    let libraries = ["libavutil", "libavcodec", "libavformat", "libavfilter"];

    for library in libraries {
        let flags = pkg_config::Config::new().probe(library).unwrap();

        for include_path in flags.include_paths {
            cfg.include(include_path);
        }

        for lib in flags.libs {
            println!("cargo:rustc-link-lib={}", lib);
        }
    }

    cfg.compile("encoder");

    // generate the needed header.
    let bindings = bindgen::Builder::default()
        .header("src/ffmpeg_encoder/libav_pixfmt.h")
        .allowlist_type("AVPixelFormat")
        .clang_arg("-I/usr/include/ffmpeg")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("libav_pixfmt.rs"))
        .expect("Could not write bindings");
}
