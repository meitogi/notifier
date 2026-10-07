# notifier

Cross-platform Rust workspace behind the `notif` CLI — native desktop
notifications with per-sender identity, callbacks and cancel-remove. Its
main consumer is the `notify/` daemon of
[meitogi/devcontainer-cli](https://github.com/meitogi/devcontainer-cli),
which dispatches through `notif` when `NOTIFY_CHANNELS` lists `notify`.

## Crates

| Crate | Role |
|---|---|
| `notif-cli` | Binary crate. Produces the `notif` executable. clap-driven CLI surface. |
| `notif-core` | Shared types (`Notification`, `Sender`, `Priority`), `Backend` trait, portable flag surface. Zero platform code. |
| `notif-macos` | `#[cfg(target_os = "macos")]` backend (UN center dispatch, sender tiers). |
| `notif-windows` | `#[cfg(target_os = "windows")]` backend (WinRT toast, AUMID, COM activator for callbacks). |
| `notif-linux` | `#[cfg(target_os = "linux")]` stub — `dispatch` is `unimplemented!`, no D-Bus dependency yet. |

## Install

- **macOS** — [docs/install-macos.md](docs/install-macos.md) (cross-compile
  from the devcontainer, first-run Gatekeeper bypass, permission dialog,
  housekeeping).
- **Windows** — shipped since 0.2.x : WinRT toasts under a per-sender
  AUMID, `--on-action` / `--on-timeout` callbacks through a COM activator.
  Build the zip with `cargo run -p notif-pack --release` (see
  [tools/pack/src/main.rs](tools/pack/src/main.rs)), unpack it on the
  host and run `notif install` once — it copies the binary into
  `%LOCALAPPDATA%\notif\` and puts that directory on the user `Path`.
- **Linux** — not implemented ; `notif-linux` is a stub.
