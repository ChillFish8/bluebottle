use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");

    // Probe + link the system libplacebo (emits the `cargo:rustc-link-*` lines)
    // and feed its include path to clang so bindgen can find the headers.
    let library = pkg_config::Config::new()
        .atleast_version("7")
        .probe("libplacebo")
        .expect("system libplacebo (>= 7) not found via pkg-config");

    // libplacebo's `vulkan.h` includes <vulkan/vulkan.h>; clang needs the
    // Khronos Vulkan headers on its include path (the `vulkan-headers` system
    // package). Probe is best-effort: when they live in a default include dir
    // clang finds them anyway.
    let vulkan = pkg_config::Config::new().probe("vulkan").ok();

    let clang_includes = library
        .include_paths
        .iter()
        .chain(vulkan.iter().flat_map(|lib| lib.include_paths.iter()))
        .map(|path| format!("-I{}", path.display()))
        .collect::<Vec<_>>();

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_args(clang_includes)
        // Bind only the libplacebo surface. The `Vk*` handle types that `pl_*`
        // Vulkan signatures reference are pulled in automatically as transitive
        // type dependencies; we deliberately do not allowlist Vulkan *functions*
        // (we link libplacebo, not the Vulkan loader — `ash` owns that).
        .allowlist_item("pl_.*")
        .allowlist_item("PL_.*")
        // libplacebo passes structs by `const *`; deriving Default lets callers
        // start from the `*_default_params` globals and override a few fields.
        .derive_default(true)
        .derive_debug(true)
        .layout_tests(false)
        .generate_cstr(true)
        .generate()
        .expect("generate libplacebo bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    bindings
        .write_to_file(&out_path)
        .expect("write generated bindings");
}
