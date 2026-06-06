// Regenerate the built-in animation sprite sheets into a directory (default:
// the repo's assets/anims/, which ships embedded and is seeded on first run).
//
//   cargo run --example gen_assets [out_dir]
//
// These generators are the *only* place the built-in art is drawn in code; the
// runtime engine just plays whatever PNG sheet a pack's .toml points at. Re-run
// this after editing a generator, then commit the PNGs.

use claude_code_launcher::anim::{blossoms, little_guy, racer, SpriteContent};
use std::path::Path;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "assets/anims".to_string());
    let dir = Path::new(&dir);
    std::fs::create_dir_all(dir).expect("create out dir");

    let save = |name: &str, sheet: SpriteContent| {
        let path = dir.join(name);
        sheet.save_sheet_png(&path).expect("save png");
        println!("wrote {}", path.display());
    };

    save("f1.png", SpriteContent::f1_car());
    save("spinner.png", SpriteContent::spinner());
    save("racer.png", racer::racer_sheet());
    save("little_guy.png", little_guy::little_guy_sheet());
    save("blossoms.png", blossoms::blossoms_sheet());
}
