//! Compiles the C++ shim over libsbml and links libsbml into the crate.
//!
//! libsbml is a C++ library whose C API is declared per class and misses
//! several functions the filler needs, so the crate reaches it through
//! `shim.cpp` rather than through generated bindings. The static library comes
//! from a prefix with `include/` and `lib/`, named by `LIBSBML_DIR` or built by
//! `scripts/build-libsbml.sh` into `third_party/prefix`.

use std::path::PathBuf;

/// The prefix of libsbml, from the environment or from `third_party/prefix`.
///
/// # Panics
///
/// When neither names a prefix which holds the headers of libsbml, since
/// nothing of the crate can be built without them.
fn prefix() -> PathBuf {
    if let Some(dir) = std::env::var_os("LIBSBML_DIR") {
        return PathBuf::from(dir);
    }
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let fallback = manifest.join("../../third_party/prefix");
    assert!(
        fallback.join("include/sbml").is_dir(),
        "set LIBSBML_DIR or run scripts/build-libsbml.sh"
    );
    fallback
}

fn main() {
    println!("cargo::rerun-if-changed=shim.cpp");
    println!("cargo::rerun-if-changed=validation.cpp");
    println!("cargo::rerun-if-env-changed=LIBSBML_DIR");

    let prefix = prefix();
    let include = prefix.join("include");
    assert!(
        include.join("sbml/SBMLTypes.h").is_file(),
        "{} holds no headers of libsbml, set LIBSBML_DIR or run scripts/build-libsbml.sh",
        prefix.display()
    );

    // NDEBUG is what libsbml itself was built with: its headers assert in the
    // code they inline, which costs a third of the time of a large read.
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .define("NDEBUG", None)
        // the classes of libsbml overload a virtual function of their base
        .flag_if_supported("-Wno-overloaded-virtual")
        .include(&include)
        .file("shim.cpp")
        .file("validation.cpp")
        .compile("sbml4humans-shim");

    // after the shim, which refers to them
    println!(
        "cargo::rustc-link-search=native={}",
        prefix.join("lib").display()
    );
    // Match libSBML's shared-library object order. Selective archive extraction
    // can choose the internal validator's weak constraint definitions before
    // the consistency validator's (notably Reaction 21152), changing diagnostics.
    println!("cargo::rustc-link-lib=static:+whole-archive=sbml-static");
    println!("cargo::rustc-link-lib=expat");
    println!("cargo::rustc-link-lib=z");
    println!("cargo::rustc-link-lib=bz2");
}
