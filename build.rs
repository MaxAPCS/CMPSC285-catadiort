//! Compiles the C++ scene files into a static library that this crate links
//! against. Nothing to change here.
//!
//! The scene files are the inputs to your renderer (scenes/ in the kit, one
//! folder up from this Cargo project) and must not be edited. They call the
//! rt* functions and the shaders, which src/rt.rs, src/shaders.rs,
//! src/transform.rs and src/noise.rs export with C linkage (#[no_mangle]
//! extern "C"), so the link resolves them in this binary. A scene of your
//! own is a new .cpp anywhere under scenes/ (picked up here automatically)
//! and one line in its folder's 0_scene_table.cpp. A later homework's folder
//! (scenes/2_Cook/, scenes/3_Kajiya/) is picked up the same way, and its
//! table is made visible to cmdline.rs through a cfg flag, as the C++ kit's
//! CMakeLists.txt does with a define.

use std::path::{Path, PathBuf};

fn cpp_files(folder: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let mut found: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    found.sort();
    for p in found {
        if p.is_dir() {
            cpp_files(&p, out);
        } else if p.extension().map_or(false, |ext| ext == "cpp") {
            out.push(p);
        }
    }
}

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let assets = Path::new(&manifest_dir).join("assets");
    let include = assets.join("include");
    let scenes = assets.join("scenes");

    let mut files: Vec<PathBuf> = Vec::new();
    cpp_files(&scenes, &mut files);
    if files.is_empty() {
        panic!("no scene files under {}", scenes.display());
    }
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
    }
    println!("cargo:rerun-if-changed={}", include.display());
    println!("cargo:rerun-if-changed={}", scenes.display());

    // Which scene tables exist besides HW2's: a cfg per folder, read by cmdline.rs.
    for (folder, flag) in [
        ("2_Cook", "rt_have_scenes_hw3"),
        ("3_Kajiya", "rt_have_scenes_hw4"),
        ("hidden", "rt_have_scenes_hidden"),
    ] {
        println!("cargo:rustc-check-cfg=cfg({})", flag);
        if scenes.join(folder).join("0_scene_table.cpp").exists() {
            println!("cargo:rustc-cfg={}", flag);
        }
    }

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .include(include)
        .files(&files)
        // The scene files are not ours to fix, so their warnings are off
        // (partially initialized parameter structs are on purpose).
        .warnings(false)
        .define("_CRT_SECURE_NO_WARNINGS", None) // MSVC: fopen and snprintf are fine
        .compile("scenes");
}
