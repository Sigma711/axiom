#[path = "build_support/source_map.rs"]
mod source_map;
use serde_json::json;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}
fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read source directory") {
        let path = entry.expect("source file").path();
        if path.is_dir() {
            files(&path, out)
        } else if path.extension().is_some_and(|s| s == "rs") {
            out.push(path);
        }
    }
}
fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=docs/book");
    println!("cargo:rerun-if-changed=build_support/source_map.rs");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");
    if let Some(dir) = git(&root, &["rev-parse", "--git-dir"]) {
        let dir = root.join(dir);
        for name in ["HEAD", "index", "refs", "packed-refs"] {
            let p = dir.join(name);
            if p.exists() {
                println!("cargo:rerun-if-changed={}", p.display());
            }
        }
    }
    let revision = git(&root, &["rev-parse", "HEAD"])
        .filter(|s| s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit()));
    let dirty = git(
        &root,
        &["status", "--porcelain", "--untracked-files=normal"],
    )
    .is_none_or(|s| !s.is_empty());
    let remote = git(&root, &["remote", "get-url", "origin"]).unwrap_or_default();
    let repository = if let Some(repo) = remote.strip_prefix("git@github.com:") {
        format!("https://github.com/{}", repo.trim_end_matches(".git"))
    } else {
        remote.trim_end_matches(".git").to_string()
    };
    let repository = if repository.starts_with("https://github.com/") {
        Some(repository)
    } else {
        None
    };
    let mut paths = Vec::new();
    files(&root.join("src"), &mut paths);
    paths.sort();
    let mut sources = BTreeMap::new();
    let mut symbols = BTreeMap::new();
    for path in paths {
        let relative = path
            .strip_prefix(&root)
            .expect("source inside project")
            .to_string_lossy()
            .replace('\\', "/");
        println!("cargo:rerun-if-changed={relative}");
        let source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {relative}: {e}"));
        for row in source_map::scan(&relative, &source)
            .unwrap_or_else(|e| panic!("AST parse {relative}: {e}"))
        {
            symbols.insert(format!("{}::{}",row.path,row.symbol),json!({"path":row.path,"symbol":row.symbol,"line":row.line,"end_line":row.end_line,"kind":row.kind}));
        }
        sources.insert(relative, source);
    }
    let data = json!({"format_version":1,"revision":revision,"dirty":dirty,"repository":repository,"symbols":symbols,"sources":sources});
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("source_map.json"),
        serde_json::to_vec(&data).unwrap(),
    )
    .expect("write build source map");
}
