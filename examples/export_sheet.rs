// Export the built-in procedural sprite sheets to real PNG + manifest files,
// so the data-driven file loader has something to consume (and so you have a
// starting asset to edit).
//
//   cargo run --example export_sheet [out_dir]   # default: /tmp/ccl-anims
//
// Then point a config at one of them:
//   [animation]
//   file = "/tmp/ccl-anims/f1.toml"

use claude_code_launcher::anim::SpriteContent;
use std::path::Path;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "/tmp/ccl-anims".to_string());
    std::fs::create_dir_all(&dir).expect("create out dir");
    let dir = Path::new(&dir);

    // (name, content, motion manifest lines mirroring the built-in registry)
    export(
        dir,
        "f1",
        SpriteContent::f1_car(),
        r#"cycle = "once"
enter_from = "left"
exit_to = "right"
rest = [0.25, 0.62, 0.0, 0.0]
fit = 0.85
min_card = 96
stiffness = 240.0
damping = 11.0
squash = 0.02
"#,
    );
    export(
        dir,
        "spinner",
        SpriteContent::spinner(),
        r#"cycle = "loop"
enter_from = "right"
exit_to = "right"
rest = [0.5, 0.5, 0.0, 0.0]
fit = 0.8
min_card = 96
stiffness = 180.0
damping = 16.0
squash = 0.02
"#,
    );

    println!("Exported sheets + manifests to {}", dir.display());
}

fn export(dir: &Path, name: &str, sprite: SpriteContent, motion: &str) {
    let png = dir.join(format!("{name}.png"));
    sprite.save_sheet_png(&png).expect("save png");

    let (fw, fh) = sprite.frame_size();
    let (ax, ay) = sprite.anchor_px();
    let play = if name == "spinner" { "loop" } else { "loop" }; // sheets loop frames
    let manifest = format!(
        "# {name} sprite-sheet animation (exported from the built-in procedural sheet)\n\
         sheet = \"{name}.png\"\n\
         frames = {frames}\n\
         columns = {cols}\n\
         frame_width = {fw}\n\
         frame_height = {fh}\n\
         fps = {fps}\n\
         anchor = {{ x = {ax}, y = {ay} }}\n\
         play = \"{play}\"\n\
         pixelated = {pix}\n\
         \n{motion}",
        frames = sprite.frame_count(),
        cols = sprite.columns(),
        fps = sprite.fps(),
        pix = sprite.is_pixelated(),
    );
    std::fs::write(dir.join(format!("{name}.toml")), manifest).expect("write manifest");
}
