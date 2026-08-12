# GekkoApp Agent Notes

## Source Of Truth

- The only Cargo package is `Gekko APP/gekkoapp-rs`; there is no root `Cargo.toml`. Run Cargo commands from that directory or use its quoted `--manifest-path`.
- Rust is the product implementation. `src/main.rs` is a thin CLI menu; package, shell, desktop, gaming, Chaotic AUR, Kito, and Bauh flows live in `src/core/flow.rs`. The Tauri v2 GUI (`cargo run --features gui --bin gekkoapp-gui`) lives in `src/gui/mod.rs` with a vanilla frontend in `ui/` (no Node: `withGlobalTauri`); it reuses the same `Reporter`-driven flows via an event-emitting reporter.
- `src/environment.rs` owns distro/session/desktop detection and the Kito compatibility gate. Put new distro capability decisions there instead of scattering `ID` checks.
- `src/kito.rs` owns Kito component metadata and selection. `src/core/github.rs` owns shared GitHub release resolution (latest + manifest asset). `src/core/catalog.rs` owns the managed-component catalog (Kito + Bauh Fork) and the pipx plan for Bauh. `src/installer.rs` owns manifest validation, downloads, archive validation, XDG paths, activation (native symlinks and `python_pipx`), and installation state.
- `docs/entorno-kito.md` describes the intended Kito contract; executable code wins if it drifts, and both must be updated together.
- `GekkoApp.sh` and `Gekko APP/GekkoApp.sh` are tracked, byte-identical launchers. If one changes, update the other and verify with `cmp -s`. They default to the Tauri Control Center (preferring `target/release/gekkoapp-gui` and otherwise compiling `--features gui`); `--cli` launches the terminal menu.
- There is no bash fallback menu: the launchers only boot the Rust binaries (GUI by default, CLI with `--cli`) and compile them with cargo on first run. Everything user-visible is Rust; keep the two launchers thin and identical. The GUI is the primary entry point for installs and updates.

## Arch And Garuda

- Product support is Arch Linux and Garuda Linux, not every distro with `ID_LIKE=arch`. Keep shared pacman behavior separate from Garuda-only helpers and repositories.
- Current development host snapshot (2026-08-11): Garuda rolling `x86_64`; repositories `garuda`, `core`, `extra`, `multilib`, and `chaotic-aur`. Re-read `/etc/os-release`, `uname -m`, and `pacman-conf --repo-list` before OS work; never bake this snapshot into product logic.
- Validate package names and repository assumptions independently on clean Arch and Garuda snapshots. Arch does not imply the `garuda` or `chaotic-aur` repositories.
- Kito currently accepts Arch-like `x86_64` + Wayland + Hyprland + systemd only. The general menu has a Niri preset, but Kito does not support Niri installation yet.
- Never introduce `pacman -Sy` or another partial-upgrade flow. For manual maintenance of a Garuda host, prefer `garuda-update`; do not use that Garuda-only command in a shared Arch path.
- Before package, service, Hyprland, or Niri changes, re-check current canonical sources rather than cached guidance: `https://wiki.archlinux.org/`, `https://man.archlinux.org/`, `https://wiki.garudalinux.org/`, `https://forum.garudalinux.org/`, `https://wiki.hypr.land/`, and `https://github.com/YaLTeR/niri`.

## Safety And UX Invariants

- Do not run `gekkoapp`, either launcher, or an interactive installer flow on the development host. Package operations, `chsh`, dotfile writes, `/etc/pacman.conf` edits, and removals belong in disposable Arch and Garuda VM snapshots.
- Run the app as a normal user; elevate individual system operations with `sudo`. Startup and preflight must remain read-only and must not prompt for sudo.
- Before any mutation, show the exact plan and packages, ask explicit `[s/N]` confirmation, and allow cancellation before the first change. Propagate cancellation/failure and never print global success after partial work.
- Preserve user files with timestamped backups. Root configuration changes require serialization, a same-filesystem atomic replacement, owner/mode preservation, checked rollback, and cleanup on every exit path.
- `run_shell*` invokes `bash -c`. Never concatenate user input, manifest values, or unvalidated paths into it; prefer `Command` with separate arguments.
- Do not add `curl | sh`, mutable remote execution, or unverified release assets. `install_bauh()` installs Bauh Fork from a signed GitHub release: HTTPS + SHA-256 manifest validation, then `pipx install --force` of the verified source artifact; it never clones a checkout or executes `install.sh`. Pipx-managed releases must declare `install_method: "python_pipx"` in the artifact manifest.
- Preserve Kito's trust boundary: resolve every selected component before mutation; require HTTPS, size and SHA-256 checks; reject unsafe archive paths/links/undeclared files; never overwrite foreign destinations.
- Kito installs artifacts and integration only; runtime wallpaper, monitor, service, and compositor behavior belongs to KiUI/CLI/Kitsune, not this installer.
- Keep user-facing UI in Spanish. For UX changes, test cancellation, narrow terminals, missing network/packages, reruns/idempotency, and honest partial-failure reporting on both distro VMs.
- Steam provider answers are currently hard-coded by menu GPU choice and depend on pacman ordering. Do not add more numeric provider assumptions; prefer explicit package/provider detection.

## GUI (Fase 2, Tauri v2)

- The `gui` Cargo feature is optional (`tauri`/`tauri-build` are optional deps) so the CLI still builds without webview toolchains. `build.rs` only invokes `tauri_build::build()` under `#[cfg(feature = "gui")]`. The `gekkoapp-gui` bin has `required-features = ["gui"]`; the CLI gate does not exercise it unless `--all-features`.
- `src/gui/mod.rs` owns the backend: `catalog_state` (local installed versions + detected environment, no network), `check_updates` (resolves the latest published version of Kito/Bauh/GekkoApp for the update bell), `install_kito`, `install_bauh`, and `install_gekkoapp` commands. They call the same `install_kito_plan`/`install_bauh`/`install_gekkoapp` flows inside `tauri::async_runtime::spawn_blocking`, with `require_confirmation = false` (the GUI already confirmed). Progress streams to the frontend as `install://event` payloads `{kind: "log"|"progress", data}`.
- GekkoApp self-update (`flow::install_gekkoapp` + `catalog::resolve_gekkoapp_plan`) reuses the release-manifest engine (`installer.rs`) with `install_method: "binary_extract"`, the native symlink layout, and no sudo (user-space XDG paths only). Because GekkoApp is commonly installed first by `scripts/install.sh` (regular files without state), the flow calls `installer::adopt_release_destinations` to reclaim those legacy paths before activation. There is no "Instalar TODO" flow anymore.
- There is no TTY in the GUI. `AskpassGuard` writes the sudo password to a 0600 temp file, creates a `SUDO_ASKPASS`/`GEKKOAPP_ASKPASS` helper, and removes both on drop. `core/system.rs` swaps to `sudo -A` only when `GEKKOAPP_ASKPASS` is set; the CLI keeps plain `sudo`. Never log or persist the password, and never extend askpass to non-sudo paths.
- Config that must stay in sync with the crate: `tauri.conf.json` (`identifier`, `version`, window `label: "main"`), `capabilities/default.json` (permissions for window `main`), `icons/icon.png`, and `ui/`. `frontendDist` is `ui`, so no frontend build step exists.

## Packaging (Fase 3)

- `scripts/install.sh` builds both binaries with `cargo build --locked --release` (+ `--features gui` for the Control Center) and installs them to `$GEKKOAPP_PREFIX/bin` (default `~/.local/bin`), the desktop entry derived from `packaging/gekkoapp-control-center.desktop` (token `__GEKKOAPP_GUI_BIN__`), and `icons/icon.png` under XDG hicolor as `org.thegekko.gekkoapp`. Idempotent; `GEKKOAPP_SKIP_BUILD=1` reinstalls without rebuilding. Never edit the generated file under `share/applications/`; edit the template.
- `scripts/build-release-bundle.sh` produces `releases/dist/gekkoapp-<version>.tar.zst` + `<version>.sha256` + `<version>.manifest.json` (contract `kitotsu.release-artifact` 1.0, `install_method: "binary_extract"`), mirroring `scripts/build-bauh-release.sh` for the Bauh Fork (`python_pipx`). The release `.desktop` `Exec` is tokenized to `@EXECUTABLE@` so the engine materializes the real launcher path at install; only the 512px PNG icon is declared in desktop integrations (the symbolic SVG ships as payload resource); the script also copies the manifest to `gekkoapp-<target>.manifest.json`, the asset name the resolver looks for.
- Fallback `install_bauh()` in the launchers downloads the manifest + archive for tag `v$TAG` from the fork release page, verifies `artifact.sha256` with `sha256sum`, then `pipx install --force` on the extracted source. Asset names must match `bauh-fork-the-gekko-<version>.tar.zst` and `bauh-fork-the-gekko-<target>.manifest.json`.
- Shell scripts are verified with `shellcheck` when available (static binary from `koalaman/shellcheck` releases if not packaged).

## Verification

- `Cargo.lock` is tracked for the binary package and release/test commands must use `--locked`. No Rust toolchain is pinned, so capture `rustc -Vv` and `cargo -V` for release evidence.
- Run this gate from `Gekko APP/gekkoapp-rs`, in order:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo build --locked --release
```

- List or focus inline tests without running the installer:

```bash
cargo test --locked -- --list
cargo test --locked environment::tests::
cargo test --locked installer::tests::installs_and_activates_a_verified_version_from_tar_zstd -- --exact
cargo test --locked pacman::tests::test_backup_guard_transitions -- --exact
```

- If launchers change, run from the repository root:

```bash
bash -n "GekkoApp.sh" "Gekko APP/GekkoApp.sh"
cmp -s "GekkoApp.sh" "Gekko APP/GekkoApp.sh"
shellcheck "GekkoApp.sh" "scripts/install.sh" "scripts/build-release-bundle.sh"
```

- There is no CI, task runner, or integration-test directory. Unit tests are local and non-privileged; system behavior still requires separate Arch and Garuda snapshot gates.

## Artifact Traps

- Launchers prefer `Gekko APP/gekkoapp-rs/target/release/gekkoapp` without checking source freshness. Build first or invoke Cargo directly; otherwise a stale executable may run.
- Root `gekkoapp`, `GekkoApp.bundle`, `patch.diff`, and `SHA256SUMS.txt` are ignored release artifacts and currently lag behind `HEAD`; never treat them as source or edit/commit them as part of a code fix.
- Do not run `git clean -fdx` casually: it deletes ignored release evidence and all Cargo output.
- Root README documents both binaries: the CLI at `Gekko APP/gekkoapp-rs/target/release/gekkoapp` and the Tauri GUI built with `--features gui`. Keep build paths pointing at the crate directory, never a root Cargo package.
- `image.png` is not the Fastfetch image: current Rust code looks for `./Anime Render.png`, which is absent. Resolve that contract explicitly when touching appearance behavior.
