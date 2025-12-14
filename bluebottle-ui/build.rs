use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    println!(
        "cargo:rerun-if-changed=assets/MaterialIcons/MaterialIconsOutlined-Regular.codepoints"
    );
    println!("cargo:rerun-if-changed=assets/MaterialIcons/MaterialIcons-Regular.codepoints");

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("icons_lut.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    let outlined_codepoints =
        include_str!("assets/MaterialIcons/MaterialIconsOutlined-Regular.codepoints");

    let mut map = phf_codegen::Map::new();
    let lines = outlined_codepoints.lines();
    for line in lines {
        let (name, code) = line.split_once(' ').expect("valid codepoint");
        map.entry(name, format!("\"\\u{{{code}}}\""));
    }
    write!(
        &mut file,
        "static OUTLINE_ICON_CODEPOINTS: phf::Map<&'static str, &'static str> = {}",
        map.build()
    )
    .unwrap();
    write!(&mut file, ";\n").unwrap();

    let filled_codepoints = include_str!("assets/MaterialIcons/MaterialIcons-Regular.codepoints");

    let mut map = phf_codegen::Map::new();
    let lines = filled_codepoints.lines();
    for line in lines {
        let (name, code) = line.split_once(' ').expect("valid codepoint");
        map.entry(name, format!("\"\\u{{{code}}}\""));
    }
    write!(
        &mut file,
        "static FILLED_ICON_CODEPOINTS: phf::Map<&'static str, &'static str> = {}",
        map.build()
    )
    .unwrap();
    write!(&mut file, ";\n").unwrap();
}
