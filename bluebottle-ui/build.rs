use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    println!(
        "cargo:rerun-if-changed=assets/MaterialIcons/MaterialSymbolsRounded[FILL,GRAD,opsz,wght].codepoints"
    );

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("icons_lut.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    let mut map = phf_codegen::Map::new();

    let codepoints =
        include_str!("assets/MaterialIcons/MaterialSymbolsRounded[FILL,GRAD,opsz,wght].codepoints");
    let lines = codepoints.lines();
    for line in lines {
        let (name, code) = line.split_once(' ').expect("valid codepoint");
        map.entry(name, format!("\"\\u{{{code}}}\""));
    }

    write!(
        &mut file,
        "static ICON_CODEPOINTS: phf::Map<&'static str, &'static str> = {}",
        map.build()
    )
    .unwrap();
    write!(&mut file, ";\n").unwrap();
}
