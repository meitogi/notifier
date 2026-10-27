# Install `notif` on Windows

`notif` dispatches WinRT toasts under a per-sender AUMID, with
`--on-action` / `--on-timeout` callbacks through a COM activator. This
guide covers a fresh install on Windows 10 / 11, ARM64 or x64.

## 1. Download and verify

From the [releases page](https://github.com/meitogi/notifier/releases),
download `SHA256SUMS` and the zip for your machine :
`notif-<version>-windows-aarch64.zip` (ARM64, e.g. Windows 11 under
Parallels on Apple Silicon) or `notif-<version>-windows-x64.zip`.

In PowerShell, from the download directory :

```powershell
(Get-FileHash .\notif-<version>-windows-aarch64.zip -Algorithm SHA256).Hash.ToLower()
Select-String notif-<version>-windows-aarch64.zip .\SHA256SUMS
```

The two hashes must be identical. (`sha256sum -c SHA256SUMS
--ignore-missing` does the same from WSL or Git Bash.)

## 2. Install

```powershell
Expand-Archive .\notif-<version>-windows-aarch64.zip -DestinationPath .
.\notif-<version>-windows-aarch64\notif.exe install
```

`notif install` copies the binary to `%LOCALAPPDATA%\notif\notif.exe`,
adds that directory to the user `Path`, registers the default sender
and shows a « notif installed » toast — that toast is the proof the
pipeline works. Open a new terminal for the `Path` change to apply,
then :

```powershell
notif send --title "notif" --body "installed"
```

Running `notif install` again after an upgrade replaces the binary ;
when nothing changed it only shows the toast again. `notif uninstall`
undoes the `Path` entry and the registration, but leaves
`%LOCALAPPDATA%\notif\notif.exe` in place (it may be the running
binary) — delete it by hand.

## 3. Building from source

From the root of this repository, inside the devcontainer :

```bash
cargo run -p notif-pack --release -- --both
# → target/dist/notif-<version>-windows-aarch64.zip
#   target/dist/notif-<version>-windows-x64.zip
#   target/dist/SHA256SUMS
```

The targets are `aarch64-pc-windows-gnullvm` and
`x86_64-pc-windows-gnullvm`, cross-compiled with `cargo zigbuild` ; add
them once with `rustup target add`. Without a flag `notif-pack` builds
aarch64 only. Unpack the zip on the Windows host and continue at § 2.
