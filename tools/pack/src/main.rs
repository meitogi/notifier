//! Post-build packager for the notif release binaries.
//!
//! Runs on the devcontainer host (and in the release workflow), invokes
//! `cargo zigbuild --release --target <triple> --bin notif` for each
//! requested target, zips the resulting binary into
//! `target/dist/notif-<version>-<os>-<arch>.zip`, then writes
//! `target/dist/SHA256SUMS` (sha256sum format) over the zips of this run.
//!
//! Invoke from the workspace root :
//!
//! ```text
//! cargo run -p notif-pack --release                 # Windows aarch64 (default)
//! cargo run -p notif-pack --release -- --aarch64    # Windows aarch64 only
//! cargo run -p notif-pack --release -- --x64        # Windows x64 only
//! cargo run -p notif-pack --release -- --both       # both Windows archs
//! cargo run -p notif-pack --release -- --macos      # both macOS archs
//! cargo run -p notif-pack --release -- --all        # the four release assets
//! cargo run -p notif-pack --release -- --no-build   # skip zigbuild, reuse binaries
//! ```

use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};

struct Target {
    triple: &'static str,
    os: &'static str,
    arch_suffix: &'static str,
    bin_name: &'static str,
}

const AARCH64: Target = Target {
    triple: "aarch64-pc-windows-gnullvm",
    os: "windows",
    arch_suffix: "aarch64",
    bin_name: "notif.exe",
};

const X64: Target = Target {
    triple: "x86_64-pc-windows-gnullvm",
    os: "windows",
    arch_suffix: "x64",
    bin_name: "notif.exe",
};

const MACOS_AARCH64: Target = Target {
    triple: "aarch64-apple-darwin",
    os: "macos",
    arch_suffix: "aarch64",
    bin_name: "notif",
};

const MACOS_X64: Target = Target {
    triple: "x86_64-apple-darwin",
    os: "macos",
    arch_suffix: "x64",
    bin_name: "notif",
};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let skip_build = args.iter().any(|a| a == "--no-build");
    let only_aarch64 = args.iter().any(|a| a == "--aarch64");
    let only_x64 = args.iter().any(|a| a == "--x64");
    let both = args.iter().any(|a| a == "--both");
    let macos = args.iter().any(|a| a == "--macos");
    let all = args.iter().any(|a| a == "--all");

    let targets: Vec<&Target> = if all {
        vec![&MACOS_AARCH64, &MACOS_X64, &AARCH64, &X64]
    } else if macos {
        vec![&MACOS_AARCH64, &MACOS_X64]
    } else {
        match (only_aarch64, only_x64, both) {
            (_, _, true) => vec![&AARCH64, &X64],
            (true, false, _) => vec![&AARCH64],
            (false, true, _) => vec![&X64],
            // Default : aarch64 only (the smoke host is ARM64).
            _ => vec![&AARCH64],
        }
    };

    let version = read_cargo_version()?;

    let mut zips = Vec::new();
    for target in targets {
        zips.push(pack_target(target, &version, skip_build)?);
    }
    write_sha256sums(&zips)
}

fn pack_target(target: &Target, version: &str, skip_build: bool) -> Result<PathBuf> {
    println!("[pack] notif v{version} → {}", target.triple);

    if !skip_build {
        println!(
            "[pack] cargo zigbuild --release --target {} --bin notif",
            target.triple,
        );
        let status = Command::new("cargo")
            .arg("zigbuild")
            .arg("--release")
            .arg("--target")
            .arg(target.triple)
            .arg("--bin")
            .arg("notif")
            .status()
            .context("spawn cargo zigbuild")?;
        if !status.success() {
            anyhow::bail!("cargo zigbuild failed for {} (exit {status})", target.triple);
        }
    } else {
        println!("[pack] --no-build : skipping cargo zigbuild for {}", target.triple);
    }

    let release_dir = PathBuf::from("target").join(target.triple).join("release");
    let exe = release_dir.join(target.bin_name);
    if !exe.exists() {
        anyhow::bail!("binary not found : {}", exe.display());
    }

    let stage_name = format!("notif-{version}-{}-{}", target.os, target.arch_suffix);
    let dist_dir = PathBuf::from("target").join("dist");
    fs::create_dir_all(&dist_dir).context("mkdir target/dist")?;

    // Wipe any prior zip for this same version + arch so a re-pack after a
    // dirty rebuild doesn't leave stale bytes.
    let zip_path = dist_dir.join(format!("{stage_name}.zip"));
    if zip_path.exists() {
        fs::remove_file(&zip_path)
            .with_context(|| format!("remove stale {}", zip_path.display()))?;
    }

    let file = fs::File::create(&zip_path)
        .with_context(|| format!("create {}", zip_path.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    // 0o755 so `unzip` on macOS restores the executable bit.
    let options: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let arc_name = format!("{stage_name}/{}", target.bin_name);
    add_file(&mut zip, &exe, &arc_name, options)?;

    zip.finish().context("finalize zip")?;

    let size = fs::metadata(&zip_path)?.len();
    println!(
        "[pack] wrote {} ({:.2} MiB, 1 file)",
        zip_path.display(),
        size as f64 / 1024.0 / 1024.0,
    );
    Ok(zip_path)
}

/// One `<hex>  <file name>` line per zip, the format `sha256sum -c` and
/// `shasum -a 256 -c` read. Overwritten on every run.
fn write_sha256sums(zips: &[PathBuf]) -> Result<()> {
    let mut out = String::new();
    for zip_path in zips {
        let bytes = fs::read(zip_path).with_context(|| format!("read {}", zip_path.display()))?;
        let name = zip_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("non-UTF-8 file name : {}", zip_path.display()))?;
        out.push_str(&format!("{}  {name}\n", hex::encode(Sha256::digest(&bytes))));
    }
    let sums_path = PathBuf::from("target").join("dist").join("SHA256SUMS");
    fs::write(&sums_path, &out).with_context(|| format!("write {}", sums_path.display()))?;
    println!("[pack] wrote {} ({} entries)", sums_path.display(), zips.len());
    Ok(())
}

fn read_cargo_version() -> Result<String> {
    let text = fs::read_to_string("Cargo.toml").context("read Cargo.toml")?;
    let mut in_workspace_package = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_workspace_package = trimmed == "[workspace.package]";
            continue;
        }
        if !in_workspace_package {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("version") {
            if let Some(v) = rest
                .split('=')
                .nth(1)
                .and_then(|s| s.trim().strip_prefix('"'))
                .and_then(|s| s.strip_suffix('"'))
            {
                return Ok(v.to_string());
            }
        }
    }
    Err(anyhow!("no version key found in [workspace.package] of Cargo.toml"))
}

fn add_file(
    zip: &mut zip::ZipWriter<fs::File>,
    src: &Path,
    arc_name: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    zip.start_file(arc_name, options).context("start_file")?;
    let mut f = fs::File::open(src).with_context(|| format!("open {}", src.display()))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).context("read")?;
    zip.write_all(&buf).context("write")?;
    Ok(())
}
