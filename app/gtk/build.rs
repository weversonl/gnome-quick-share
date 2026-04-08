use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=data/");
    println!("cargo:rerun-if-changed=po/");

    let out_dir = std::env::var("OUT_DIR").unwrap();

    // ── Compile GSettings schema ──────────────────────────────────────────────
    let schema_status = Command::new("glib-compile-schemas")
        .arg(format!("--targetdir={out_dir}"))
        .arg("data/")
        .status();

    match schema_status {
        Ok(s) if s.success() => {}
        Ok(_) => panic!("glib-compile-schemas failed"),
        Err(e) => panic!("glib-compile-schemas not found: {e} — install glib2 package"),
    }

    println!("cargo:rustc-env=SCHEMA_DIR={out_dir}");

    // ── Compile .po → .mo files ───────────────────────────────────────────────
    for lang in &["en", "pt_BR"] {
        let po_path = format!("po/{lang}.po");
        if !std::path::Path::new(&po_path).exists() {
            continue;
        }
        let mo_dir = PathBuf::from(&out_dir)
            .join("locale")
            .join(lang)
            .join("LC_MESSAGES");
        std::fs::create_dir_all(&mo_dir).unwrap();
        let mo_path = mo_dir.join("gnomeqs.mo");

        let status = Command::new("msgfmt")
            .args(["-o", mo_path.to_str().unwrap(), &po_path])
            .status();

        match status {
            Ok(s) if s.success() => {}
            Ok(_) => panic!("msgfmt failed for {lang}"),
            Err(e) => panic!("msgfmt not found or failed for {lang}: {e}"),
        }
    }

    println!("cargo:rustc-env=LOCALE_DIR={out_dir}/locale");
}
