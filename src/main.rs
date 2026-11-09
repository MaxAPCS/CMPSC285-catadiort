//! main.rs: render one scene by name (gauchoRT Spheres; gauchoRT alone lists
//! them) or a set of them (gauchoRT all; gauchoRT stills, without the
//! movies; gauchoRT test | benchmark | study | compare | hero | extra, one
//! tier). The command line is read in cmdline.rs (usage() there lists every
//! option) into OPTIONS (options.rs), which the renderer reads at
//! rtWorldEnd. Complete in both kits; the one edit is the SIMPLE_MAIN switch
//! below; the twin of the C++ kit's main.cpp. The executable changes to the
//! kit root by itself (textures/, models/, renders/).

mod api;
mod cmdline;
mod engine;
mod options;
mod utils;

use std::path::Path;

use options::OPTIONS;

fn main() {
    // The scenes look for textures/ and models/ and write into renders/
    // relative to the kit root, the folder above this Cargo project. Go
    // there, so that `cargo run` works from rust/ or from the kit root.

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let assets = Path::new(&manifest_dir).join("assets");
    let scenes = assets.join("scenes");

    if scenes.is_dir() {
        let _ = std::env::set_current_dir(manifest_dir);
    }

    let tables = cmdline::tables();

    // The options go into OPTIONS; what comes back is the scene argument,
    // one scene's name or a set of them (all, stills, or a tier). --help, a
    // missing scene and the tests --warptest and --bsdftest exit in there.
    let args: Vec<String> = std::env::args().collect();
    let scene = cmdline::parse_command_line(&args, &tables);

    // Run every scene the argument selects: one table per folder of scenes/
    // the build found, each an array of {name, function, tier} entries.
    let mut ran = 0;
    for table in &tables {
        for s in &table.scenes {
            if cmdline::selected(&scene, s, table.hw) {
                println!("== {}", s.name);
                OPTIONS.write().expect("options").scene = s.name.to_string(); /* for the TIME line */
                // Render the scene
                // SAFETY: the C++ scene functions take no arguments and return nothing.
                unsafe { (s.function)() };
                ran += 1;
            }
        }
    }

    if ran == 0 {
        eprintln!("unknown scene '{}'", scene);
        cmdline::usage(&tables);
        std::process::exit(1);
    }
}

#[cfg(any(
    feature = "aovraydir",
    feature = "aovdepth",
    feature = "aovnormal",
    feature = "aovalbedo"
))]
const _: () = {
    let count = cfg!(feature = "aovraydir") as u8
        + cfg!(feature = "aovdepth") as u8
        + cfg!(feature = "aovnormal") as u8
        + cfg!(feature = "aovalbedo") as u8;
    if count > 1 {
        panic!("at most one of the `aov*` features may be enabled at a time");
    }
};
