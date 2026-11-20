use anyhow::Result;
use std::{fs, path::Path, process::Command};

/// Write the project version to `version.txt` in the output directory, following the scheme in
/// `plans/README.md`: `<VERSION>` for a commit tagged `v<VERSION>`, otherwise
/// `<VERSION>-dev.<commit UTC date>.<commit UTC time>+<short hash>`, or `<VERSION>-dev` without git.
/// An unchanged file is not rewritten, so watchers do not see a change.
pub fn write_version(root: &Path, output_directory: &Path) -> Result<()> {
    let release = fs::read_to_string(root.join("VERSION"))?.trim().to_owned();
    let mut version = format!("{release}-dev");
    if let (Some(tags), Some(time), Some(hash)) = (
        run_git(root, &["tag", "--points-at", "HEAD"]),
        run_git(
            root,
            &[
                "show",
                "-s",
                "--format=%cd",
                "--date=format-local:%Y%m%d.%H%M%S",
                "HEAD",
            ],
        ),
        run_git(root, &["rev-parse", "--short", "HEAD"]),
    ) {
        if tags.lines().any(|tag| tag == format!("v{release}")) {
            version = release;
        } else {
            version = format!("{release}-dev.{time}+{hash}");
        }
    }
    let path = output_directory.join("version.txt");
    if fs::read_to_string(&path).ok().as_deref() != Some(version.as_str()) {
        fs::write(path, version)?;
    }
    Ok(())
}

/// Run git in the repository, returning its trimmed output, or `None` if it fails.
fn run_git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("TZ", "UTC")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_owned())
}
