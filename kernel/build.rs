use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    println!("cargo:rustc-link-arg=-Tlinker-{arch}.ld");
    println!("cargo:rerun-if-changed=linker-{arch}.ld");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let music_dir = Path::new(&manifest_dir).join("src/apps/music");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest_path = Path::new(&out_dir).join("songs_generated.rs");

    let mut entries: Vec<(String, String)> = Vec::new();

    if music_dir.is_dir() {
        let read_dir = fs::read_dir(&music_dir)
            .unwrap_or_else(|e| panic!("не смог прочитать {}: {}", music_dir.display(), e));

        for entry in read_dir {
            let entry = entry.expect("плохая запись в директории music/");
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("raw") {
                continue;
            }

            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("song")
                .to_string();

            let abs_path = path
                .canonicalize()
                .unwrap_or_else(|e| panic!("не смог получить абсолютный путь {}: {}", path.display(), e))
                .to_string_lossy()
                .to_string();

            println!("cargo:rerun-if-changed={}", abs_path);

            entries.push((name, abs_path));
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::new();
    out.push_str("// автоген build.rs из src/apps/music/*.raw - не редактировать руками\n");
    out.push_str("pub static SONGS: &[(&str, &[u8])] = &[\n");
    for (name, path) in &entries {
        out.push_str(&format!("    ({name:?}, include_bytes!({path:?})),\n"));
    }
    out.push_str("];\n");

    fs::write(&dest_path, out)
        .unwrap_or_else(|e| panic!("не смог записать {}: {}", dest_path.display(), e));

    println!("cargo:rerun-if-changed={}", music_dir.display());
}
