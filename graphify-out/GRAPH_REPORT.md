# Graph Report - GekkoApp  (2026-08-23)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 424 nodes · 1117 edges · 22 communities (16 shown, 6 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 52 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `e9b67503`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- installer.rs
- Reporter
- gui/mod.rs
- app.js
- csp
- github.rs
- pacman.rs
- theme.rs
- CliReporter
- .from_sources
- kito.rs
- install-release.sh
- default.json
- install.sh
- Gekko APP/GekkoApp.sh
- GekkoApp.sh
- build-bauh-release.sh
- build-release-bundle.sh
- gekkoapp

## God Nodes (most connected - your core abstractions)
1. `Reporter` - 37 edges
2. `activate_desktop_integration()` - 20 edges
3. `ArtifactManifest` - 18 edges
4. `io_error()` - 17 edges
5. `SystemEnvironment` - 17 edges
6. `run_gui_install()` - 16 edges
7. `activate_release()` - 15 edges
8. `InstallPaths` - 15 edges
9. `GuiReporter` - 15 edges
10. `PreparedRelease` - 14 edges

## Surprising Connections (you probably didn't know these)
- `download_manifest_body()` --calls--> `download_bytes()`  [INFERRED]
  Gekko APP/gekkoapp-rs/src/core/github.rs → Gekko APP/gekkoapp-rs/src/installer.rs
- `install_chaotic_aur()` --calls--> `check_arch_linux()`  [INFERRED]
  Gekko APP/gekkoapp-rs/src/core/pacman.rs → Gekko APP/gekkoapp-rs/src/core/system.rs
- `install_chaotic_aur()` --calls--> `run_shell()`  [INFERRED]
  Gekko APP/gekkoapp-rs/src/core/pacman.rs → Gekko APP/gekkoapp-rs/src/core/system.rs
- `install_chaotic_aur()` --calls--> `run_shell_piped()`  [INFERRED]
  Gekko APP/gekkoapp-rs/src/core/pacman.rs → Gekko APP/gekkoapp-rs/src/core/system.rs
- `catalog_state()` --calls--> `all_components()`  [INFERRED]
  Gekko APP/gekkoapp-rs/src/gui/mod.rs → Gekko APP/gekkoapp-rs/src/core/catalog.rs

## Import Cycles
- None detected.

## Communities (22 total, 6 thin omitted)

### Community 0 - "installer.rs"
Cohesion: 0.09
Nodes (80): BTreeSet, Component, Error, FnOnce, activate_desktop_integration(), activate_pipx_release(), activate_release(), activate_symlink() (+72 more)

### Community 1 - "Reporter"
Cohesion: 0.09
Nodes (59): build_zshrc(), confirm_or_override_environment(), install_bauh(), install_gaming(), install_gaming_solus(), install_gekko_adb(), install_gekkoapp(), install_hyprland() (+51 more)

### Community 2 - "gui/mod.rs"
Cohesion: 0.11
Nodes (44): AppHandle, AskpassGuard, catalog_state(), catalog_state_reports_all_components(), CatalogItem, CatalogView, check_updates(), GuiReporter (+36 more)

### Community 3 - "app.js"
Cohesion: 0.18
Nodes (22): appendLog(), applyPalette(), badge(), barFill, bellBadge, bellMenu, cardFor(), init() (+14 more)

### Community 4 - "csp"
Cohesion: 0.09
Nodes (22): app, security, windows, withGlobalTauri, build, frontendDist, bundle, active (+14 more)

### Community 5 - "github.rs"
Cohesion: 0.33
Nodes (10): Agent, download_manifest_body(), GithubAsset, GithubRelease, resolve_latest_release(), BTreeMap, Result, String (+2 more)

### Community 6 - "pacman.rs"
Cohesion: 0.19
Nodes (13): BackupGuard, check_chaotic_aur_configured(), default_remover(), install_chaotic_aur(), LockGuard, LockGuard<'a>, replace_pacman_conf_securely(), ReplaceResult (+5 more)

### Community 7 - "theme.rs"
Cohesion: 0.17
Nodes (19): detect_palette(), is_dark_palette(), MatugenPalette, modified_time(), palette_path(), parse_define_colors(), parses_define_colors(), relative_luminance() (+11 more)

### Community 8 - "CliReporter"
Cohesion: 0.14
Nodes (7): CliReporter, fake_progress_bar(), hide_cursor(), print_header(), read_line(), String, show_cursor()

### Community 9 - ".from_sources"
Cohesion: 0.24
Nodes (13): command_exists(), Compatibility, detect_desktop(), detect_package_manager(), detect_session(), detects_solus_environment_with_eopkg(), detects_supported_arch_hyprland_environment(), parse_os_release() (+5 more)

### Community 10 - "kito.rs"
Cohesion: 0.11
Nodes (19): all_components(), CatalogComponent, resolve_bauh_plan(), resolve_gekkoapp_plan(), resolves_published_bauh_release_from_github(), Result, String, Vec (+11 more)

### Community 11 - "install-release.sh"
Cohesion: 0.42
Nodes (8): fail(), info(), refresh_desktop_db(), require(), say(), install-release.sh script, uninstall(), usage()

### Community 12 - "default.json"
Cohesion: 0.25
Nodes (7): description, identifier, permissions, $schema, windows, core:default, main

## Knowledge Gaps
- **33 isolated node(s):** `description`, `identifier`, `$schema`, `core:default`, `main` (+28 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **6 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Reporter` connect `Reporter` to `CliReporter`, `gui/mod.rs`, `pacman.rs`?**
  _High betweenness centrality (0.222) - this node is a cross-community bridge._
- **Why does `theme_state()` connect `theme.rs` to `gui/mod.rs`?**
  _High betweenness centrality (0.075) - this node is a cross-community bridge._
- **What connects `description`, `identifier`, `$schema` to the rest of the system?**
  _33 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `installer.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.08602150537634409 - nodes in this community are weakly interconnected._
- **Should `Reporter` be split into smaller, more focused modules?**
  _Cohesion score 0.09326923076923077 - nodes in this community are weakly interconnected._
- **Should `gui/mod.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.1081239041496201 - nodes in this community are weakly interconnected._
- **Should `csp` be split into smaller, more focused modules?**
  _Cohesion score 0.08695652173913043 - nodes in this community are weakly interconnected._