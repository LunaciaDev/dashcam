fn main() {
    let mut cfg = cc::Build::new();
    cfg.file("src/encoder.c");

    let libraries = ["libavutil"];

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
}
