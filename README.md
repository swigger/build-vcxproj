# Load information from vcxproj in build.rs

I'm using rust and cpp to work together. The worker uses visual studio to compile the C++ code into a static library and link it to the rust program.
But there is a problem. In order to allow the rust program to be compiled independently, I need to specify a lot of information in build.rs such as compilation options and source file lists. To make build.rs easier to write, I implemented this library.

It can read file list and other information from vcxproj, and provide common compilation options according to version configuration. This makes build.rs shorter and easier to implement, so that the source file list does not need to be updated in two places.

## Example build.rs

```rust
fn main() {
    env::set_var("VSLANG", "1033");
    let from_vs = env::var("VisualStudioDir").map(|x| !x.is_empty()).unwrap_or(false);
    let is_debug = env::var("PROFILE").map(|x| x == "debug").unwrap_or(false);
    let mut proj = Vcxproj::new("vs.proj/example.vcxproj", is_debug);
    proj.load_config();

    for ld in &proj.lib_dirs {
        println!("cargo:rustc-link-search=native={}", ld);
    }

    if from_vs || Path::new(&proj.target_fn).is_file() {
        println!("cargo:rerun-if-changed={}", proj.target_fn);
        println!("cargo:rustc-link-lib=static={}", proj.target);
    } else {
        for srcfile in &proj.sources {
            println!("cargo:rerun-if-changed={}", srcfile);
        }
        for header in glob::glob("src/**/*.h").unwrap() {
            println!("cargo:rerun-if-changed={}", header.unwrap().display());
        }

        let mut cco = cc::Build::new();
        cco.cpp(true).std("c++20").files(&proj.sources);
        for inc in &proj.include_dirs {
            cco.flag(&format!("-I{}", inc));
        }
        for f in &proj.flags {
            cco.flag(f);
        }
        cco.compile(&proj.basename());
    }
    if proj.find_lib("utilwin") {
        println!("cargo:rustc-link-lib=utilwin");
    }
    // add other libraries
}
```

