use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) {
    println!("cargo:rerun-if-changed={}", directory.display());
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs")
            || path.file_name().is_some_and(|name| name == "Cargo.toml")
        {
            files.push(path);
        }
    }
}

fn write_inputs(root: &Path, source: &mut String, name: &str, inputs: &[&str]) {
    let mut files = Vec::new();
    for input in inputs {
        let path = root.join(input);
        if path.is_dir() {
            collect_files(&path, &mut files);
        } else {
            files.push(path);
        }
    }
    files.sort();
    source.push_str(&format!("const {name}: &[(&str, &[u8])] = &[\n"));
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        source.push_str(&format!(
            "({:?}, include_bytes!({:?})),\n",
            path.strip_prefix(root).unwrap(),
            path
        ));
    }
    source.push_str("];\n");
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    let mut source = String::new();
    write_inputs(
        &root,
        &mut source,
        "SHARED_CODE_INPUTS",
        &["Cargo.toml", "Cargo.lock", "crates/catalog_builder"],
    );
    write_inputs(
        &root,
        &mut source,
        "LOGIC_CODE_INPUTS",
        &["crates/logic_catalog"],
    );
    write_inputs(
        &root,
        &mut source,
        "RETILING_CODE_INPUTS",
        &["crates/retiling_catalog"],
    );
    write_inputs(
        &root,
        &mut source,
        "PATCH_CODE_INPUTS",
        &["crates/logic_catalog", "crates/patch_catalog"],
    );
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("code_inputs.rs"),
        source,
    )
    .unwrap();
}
