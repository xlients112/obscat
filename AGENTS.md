# AGENTS.md

Repo-specific instructions for AI agents working in this repository.

## Project

`xserve` is a Rust container manager: a gRPC client CLI (`xserve`) plus a gRPC daemon (`xserved`) that drive containers through the external [`youki`](https://github.com/youki-dev/youki) runtime CLI.

Cargo workspace at the repo root, members `xserve` and `xserved`:

- `xserve` — CLI client (clap). Subcommands: `version`, `pull`, `spec`, `create`, `delete`, `state`, `run`, `kill`.
- `xserved` — gRPC daemon (tonic + prost). Serves `ContainerService` on `0.0.0.0:5005`.

The client talks to the daemon over gRPC at a static endpoint: `http://127.0.0.1:5005`.

## Commands

Run from the workspace root (root `Cargo.toml` declares `members = ["xserve", "xserved"]`).

- `cargo check --workspace` — fast compile check; this is the primary verification command.
- `cargo build` or `cargo build -p xserve` / `cargo build -p xserved` — build the workspace or a single crate.
- `cargo run -p xserved` — run the daemon locally.
- `cargo run -p xserve -- --version` — run the client.

Gotchas:

- `protoc` (protobuf-compiler) must be installed; the `tonic-prost-build` build scripts fail without it (`error: Could not find 'protoc'`).
- Rust edition 2024 is required (rustc 1.99.1 in this environment).
- No tests, CI, or lint/format configuration exist. `cargo check --workspace` passing is the current bar.

## Key files

- `proto/xserve.proto` — gRPC service `ContainerService` (package `xserve`); RPCs: Spec, Create, Delete, State, Run, Kill. Codegen via `build.rs` using `tonic_prost_build::configure().compile_protos(&["proto/xserve.proto"], &["proto"])`; included as `tonic::include_proto!("xserve")`.
- `xserve/src/main.rs` — clap CLI dispatch.
- `xserve/src/daemon/` — client-side gRPC request wrappers (`internal.rs`) and Docker Hub image pull (`image.rs`: token → manifest → layers → unpack tar.gz to `/var/lib/xserved/bundles/<id>/rootfs`).
- `xserved/src/main.rs` — tonic server impl; each RPC spawns a `tokio::spawn` task.
- `xserved/src/runtime/` — runtime orchestration: `internal.rs` (`youki` CLI calls), `network.rs` (Linux bridge `xsvd0` @ 172.19.0.1/16, veth, `nsenter`, container IP 172.19.0.2/16), `logging.rs`.

## Quirks worth knowing

- gRPC response field is misspelled `sucess` in the proto and all usages — keep the spelling consistent or cross-crate compile checks will fail.
- Container lifecycle is delegated to the external `youki` binary; requires root plus `ip` / `nsenter` (iproute2, util-linux) and the `youki` binary installed (see `setup.sh`).
- Daemon listens on `0.0.0.0:5005`; client always targets `127.0.0.1:5005`.
- Supported platforms: Linux only (aarch64, x86_64); project is in dev/pre-release status.
- Empty placeholder modules: `xserved/src/runtime/socket.rs`, `stream.rs`.

## Setup

- `xserve/setup.sh` requires root; installs iproute2/util-linux/curl/tar/libseccomp, downloads the `youki` runtime and release binaries, and registers a systemd service for the daemon.

## Layout notes

- Workspace root is the repo root; each crate dir (`xserve/`, `xserved/`) also contains its own nested `.git` and lockfile — the root lockfile governs the workspace.
