"use strict";

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args || {});
const listen = (event, handler) => window.__TAURI__.event.listen(event, handler);

const $ = (id) => document.getElementById(id);
const logEl = $("log");
const barFill = $("bar-fill");
let catalog = null;
let unlisten = null;

function appendLog(level, message) {
  const line = document.createElement("div");
  line.className = level;
  line.textContent = `[${new Date().toLocaleTimeString()}] ${message}`;
  logEl.appendChild(line);
  logEl.scrollTop = logEl.scrollHeight;
}

function setProgress(label, percent) {
  $("progress-label").textContent = label || "";
  barFill.style.width = `${Math.max(0, Math.min(100, percent))}%`;
}

function badge(text, kind) {
  const el = document.createElement("span");
  el.className = "badge" + (kind ? ` ${kind}` : "");
  el.textContent = text;
  return el;
}

function installedOf(products) {
  const versions = new Map();
  for (const item of catalog.items) {
    if (products.has(item.id) && item.installedVersion) {
      versions.set(item.id, item.installedVersion);
    }
  }
  return versions;
}

function render() {
  const env = catalog;
  const envText =
    `${env.distroName} · ${env.desktop} · ${env.session} · ${env.target || "sin target"}` +
    (env.compatible ? "" : " · NO COMPATIBLE");
  $("env").textContent = envText;

  const mandatory = catalog.kitoModules.filter((m) => m.mandatory);
  const optional = catalog.kitoModules.filter((m) => !m.mandatory);
  const installedMandatory = mandatory.filter((m) => m.installedVersion);

  const kitoStatus = $("kito-status");
  if (installedMandatory.length === mandatory.length && mandatory.length > 0) {
    kitoStatus.replaceChildren(badge(`${installedMandatory.length}/${mandatory.length} base`, "installed"));
  } else {
    kitoStatus.replaceChildren(badge("no instalado", "error"));
  }

  const modules = $("kito-modules");
  modules.replaceChildren();
  for (const m of [...mandatory, ...optional]) {
    const row = document.createElement("div");
    row.className = "module";

    const label = document.createElement("label");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = m.mandatory || false;
    box.disabled = m.mandatory;
    box.id = `mod-${m.productId}`;
    box.dataset.product = m.productId;
    label.appendChild(box);
    label.appendChild(document.createTextNode(m.label));
    row.appendChild(label);

    const version = document.createElement("span");
    version.className = "version";
    version.textContent = m.mandatory ? "(obligatorio)" : (m.installedVersion || "no instalado");
    row.appendChild(version);

    modules.appendChild(row);
  }
  refreshButtons();

  const bauh = catalog.items.find((i) => i.id === "bauh-fork-the-gekko");
  const bauhStatus = $("bauh-status");
  if (bauh && bauh.installedVersion) {
    bauhStatus.replaceChildren(badge(`v${bauh.installedVersion}`, "installed"));
  } else {
    bauhStatus.replaceChildren(badge("no instalado", "error"));
  }

  const gekkoAdb = catalog.items.find((i) => i.id === "gekko-adb");
  const gekkoAdbStatus = $("gekko-adb-status");
  if (gekkoAdb && gekkoAdb.installedVersion) {
    gekkoAdbStatus.replaceChildren(badge(gekkoAdb.installedVersion, "installed"));
  } else {
    gekkoAdbStatus.replaceChildren(badge("no instalado", "error"));
  }

  const gekkoapp = catalog.items.find((i) => i.id === "gekkoapp");
  const gekkoappStatus = $("gekkoapp-status");
  if (gekkoapp && gekkoapp.installedVersion) {
    gekkoappStatus.replaceChildren(badge(`v${gekkoapp.installedVersion}`, "installed"));
  } else {
    gekkoappStatus.replaceChildren(badge("no instalado", "error"));
  }
}

function refreshButtons() {
  $("kito-install").disabled = $("kito-pass").value.trim() === "";
  $("bauh-install").disabled = $("bauh-pass").value.trim() === "";
  $("gekko-adb-install").disabled = $("gekko-adb-pass").value.trim() === "";
  $("terminal-install").disabled = $("terminal-pass").value.trim() === "";
  $("hyprland-install").disabled = $("hyprland-pass").value.trim() === "";
  $("niri-install").disabled = $("niri-pass").value.trim() === "";
  $("gaming-install").disabled = $("gaming-pass").value.trim() === "";
  $("chaotic-install").disabled = $("chaotic-pass").value.trim() === "";
}

// ── Campana de actualizaciones ───────────────────────────────────────────────

let updates = [];
const bellMenu = $("bell-menu");
const bellBadge = $("bell-badge");

function cardFor(id) {
  if (["kitsune-compositor", "kiui", "kitowall", "kilivepaper", "kisddm"].includes(id)) {
    return $("kito-card");
  }
  if (id === "bauh-fork-the-gekko") return $("bauh-card");
  if (id === "gekkoapp") return $("gekkoapp-card");
  return null;
}

function renderBell() {
  const pending = updates.filter((u) => u.updateAvailable);
  bellBadge.textContent = pending.length;
  bellBadge.classList.toggle("hidden", pending.length === 0);
  bellMenu.replaceChildren();
  if (pending.length === 0) {
    const none = document.createElement("div");
    none.className = "update-item muted";
    none.textContent = "Todo actualizado";
    bellMenu.appendChild(none);
    return;
  }
  for (const u of pending) {
    const item = document.createElement("button");
    item.type = "button";
    item.className = "update-item";
    const name = document.createElement("span");
    name.className = "update-name";
    name.textContent = u.label;
    const versions = document.createElement("span");
    versions.className = "update-versions";
    versions.textContent = `${u.installed || "—"} → ${u.latest}`;
    item.appendChild(name);
    item.appendChild(versions);
    item.addEventListener("click", () => {
      bellMenu.classList.add("hidden");
      const card = cardFor(u.id);
      if (card) card.scrollIntoView({ behavior: "smooth", block: "center" });
    });
    bellMenu.appendChild(item);
  }
}

async function refreshUpdates() {
  try {
    updates = await invoke("check_updates");
  } catch {
    updates = [];
  }
  renderBell();
}

function mixHex(a, b, t) {
  const pa = parseInt(a.slice(1), 16);
  const pb = parseInt(b.slice(1), 16);
  const r = Math.round(((pa >> 16) & 255) * (1 - t) + ((pb >> 16) & 255) * t);
  const g = Math.round(((pa >> 8) & 255) * (1 - t) + ((pb >> 8) & 255) * t);
  const bl = Math.round((pa & 255) * (1 - t) + (pb & 255) * t);
  return "#" + ((1 << 24) | (r << 16) | (g << 8) | bl).toString(16).slice(1);
}

function paletteColor(p, name, fallback) {
  return (p && p.colors && p.colors[name]) || fallback;
}

function renderThemeStatus(p) {
  const status = $("theme-status");
  const desc = $("theme-desc");
  const swatches = $("theme-swatches");
  if (p && p.available) {
    status.replaceChildren(badge(p.dark ? "matugen · oscuro" : "matugen · claro", "installed"));
    desc.textContent = `Siguiendo la paleta: ${p.source}`;
    const keys = [
      ["window_bg_color", "fondo"],
      ["card_bg_color", "tarjeta"],
      ["accent_color", "acento"],
      ["window_fg_color", "texto"],
    ];
    const chips = keys.map(([key, label]) => {
      const chip = document.createElement("span");
      chip.className = "swatch";
      chip.style.background = paletteColor(p, key, "#888");
      chip.title = `${label} · ${paletteColor(p, key, "")}`;
      return chip;
    });
    swatches.replaceChildren(...chips);
  } else {
    status.replaceChildren(badge("tema por defecto", ""));
    desc.textContent =
      "Sin paleta de matugen: GekkoApp usa el tema oscuro por defecto. Cambia el wallpaper con tu setup de HyDE/QuickShell para re-generar la paleta.";
    swatches.replaceChildren();
  }
}

function applyPalette(p) {
  const root = document.documentElement.style;
  const fallback = {
    bg: "#0f1115",
    fg: "#e6e9f0",
    panel: "#161a22",
    panel2: "#1c2230",
    muted: "#8b93a7",
    accent: "#5f8bff",
    red: "#f0676b",
  };
  if (!p || !p.available) {
    for (const key of ["--bg", "--panel", "--panel-2", "--border", "--text", "--muted", "--accent", "--red"]) {
      root.removeProperty(key);
    }
    document.documentElement.dataset.theme = "dark";
    renderThemeStatus(null);
    return;
  }
  const bg = paletteColor(p, "window_bg_color", paletteColor(p, "theme_bg_color", fallback.bg));
  const fg = paletteColor(p, "window_fg_color", paletteColor(p, "theme_fg_color", fallback.fg));
  const panel = paletteColor(p, "card_bg_color", paletteColor(p, "sidebar_bg_color", fallback.panel));
  const panel2 = paletteColor(p, "popover_bg_color", paletteColor(p, "view_bg_color", fallback.panel2));
  const muted = paletteColor(p, "sidebar_fg_color", fallback.muted);
  const accent = paletteColor(p, "accent_color", paletteColor(p, "accent_bg_color", fallback.accent));
  const red = paletteColor(p, "destructive_color", fallback.red);
  root.setProperty("--bg", bg);
  root.setProperty("--panel", panel);
  root.setProperty("--panel-2", panel2);
  root.setProperty("--border", mixHex(panel, fg, 0.18));
  root.setProperty("--text", fg);
  root.setProperty("--muted", muted);
  root.setProperty("--accent", accent);
  root.setProperty("--red", red);
  document.documentElement.dataset.theme = p.dark ? "dark" : "light";
  renderThemeStatus(p);
}

async function loadTheme() {
  const palette = await invoke("theme_state");
  applyPalette(palette);
}

async function runInstall(command, args) {
  setProgress("Iniciando...", 5);
  if (unlisten) {
    unlisten.then((fn) => fn());
    unlisten = null;
  }
  unlisten = listen("install://event", (event) => {
    const payload = event.payload;
    if (payload.kind === "log") {
      appendLog(payload.data.level, payload.data.message);
    } else if (payload.kind === "progress") {
      setProgress(payload.data.label, payload.data.percent);
    }
  });

  try {
    const result = await invoke(command, args);
    appendLog("ok", "Operacion completada.");
    setProgress("Completado", 100);
    catalog = await invoke("catalog_state");
    render();
    refreshUpdates();
    return result;
  } catch (error) {
    appendLog("err", String(error));
    setProgress("Fallo", 100);
    throw error;
  }
}

async function init() {
  catalog = await invoke("catalog_state");
  render();
  await loadTheme();

  $("kito-pass").addEventListener("input", refreshButtons);
  $("bauh-pass").addEventListener("input", refreshButtons);
  $("gekko-adb-pass").addEventListener("input", refreshButtons);
  $("terminal-pass").addEventListener("input", refreshButtons);
  $("hyprland-pass").addEventListener("input", refreshButtons);
  $("niri-pass").addEventListener("input", refreshButtons);
  $("gaming-pass").addEventListener("input", refreshButtons);
  $("chaotic-pass").addEventListener("input", refreshButtons);

  listen("theme://changed", (event) => applyPalette(event.payload));

  $("kito-install").addEventListener("click", () => {
    const selection = {
      kitowall: $("mod-kitowall").checked,
      kilivepaper: $("mod-kilivepaper").checked,
      kisddm: $("mod-kisddm").checked,
    };
    runInstall("install_kito", { selection, password: $("kito-pass").value });
  });

  $("bauh-install").addEventListener("click", () => {
    runInstall("install_bauh", { password: $("bauh-pass").value });
  });

  $("gekko-adb-install").addEventListener("click", () => {
    runInstall("install_gekko_adb", { password: $("gekko-adb-pass").value });
  });

  $("gekkoapp-install").addEventListener("click", () => {
    runInstall("install_gekkoapp", {});
  });

  $("terminal-install").addEventListener("click", () => {
    runInstall("install_terminal", { password: $("terminal-pass").value });
  });

  $("hyprland-install").addEventListener("click", () => {
    runInstall("install_hyprland", { password: $("hyprland-pass").value });
  });

  $("niri-install").addEventListener("click", () => {
    runInstall("install_niri", { password: $("niri-pass").value });
  });

  $("gaming-install").addEventListener("click", () => {
    runInstall("install_gaming_setup", {
      gpu: $("gaming-gpu").value,
      password: $("gaming-pass").value,
    });
  });

  $("chaotic-install").addEventListener("click", () => {
    runInstall("install_chaotic_aur", { password: $("chaotic-pass").value });
  });

  $("bell-toggle").addEventListener("click", (event) => {
    event.stopPropagation();
    bellMenu.classList.toggle("hidden");
  });
  document.addEventListener("click", () => bellMenu.classList.add("hidden"));
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") bellMenu.classList.add("hidden");
  });

  refreshUpdates();
}

init().catch((error) => {
  appendLog("err", `No se pudo iniciar el catalogo: ${error}`);
});
