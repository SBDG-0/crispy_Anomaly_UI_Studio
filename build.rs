use std::{env, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");
    println!("cargo:rerun-if-changed=assets/app.rc");
    let target = env::var("TARGET").unwrap();
    if !target.contains("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let (tool, args, resource) = if target.contains("gnu") {
        let file = out.join("app-icon.o");
        (
            "windres",
            vec![
                "-i".into(),
                "assets/app.rc".into(),
                "-o".into(),
                file.to_string_lossy().into_owned(),
            ],
            file,
        )
    } else {
        let file = out.join("app-icon.res");
        (
            "rc",
            vec![
                "/nologo".into(),
                format!("/fo{}", file.display()),
                "assets/app.rc".into(),
            ],
            file,
        )
    };
    let result = Command::new(tool).args(args).status().expect(
        "Install windres (GNU toolchain) or Windows SDK rc (MSVC toolchain) to embed the app icon",
    );
    assert!(
        result.success(),
        "Application icon resource compilation failed"
    );
    println!("cargo:rustc-link-arg={}", resource.display());
}
