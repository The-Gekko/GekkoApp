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

// ── Confirmacion previa a cualquier mutacion ─────────────────────────────────
// El backend recibe require_confirmation=false y GuiReporter::confirm() acepta
// siempre, asi que la confirmacion explicita del usuario tiene que ocurrir aqui.
// Sin esto un clic en "Desinstalar" borraba cosas sin preguntar.
function confirmAction({ title, body, detail, confirmLabel, danger }) {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.className = "modal-overlay";

    const box = document.createElement("div");
    box.className = "modal";
    box.setAttribute("role", "dialog");
    box.setAttribute("aria-modal", "true");

    const heading = document.createElement("h3");
    heading.textContent = title;
    box.appendChild(heading);

    const text = document.createElement("p");
    text.className = "modal-body";
    text.textContent = body;
    box.appendChild(text);

    if (detail) {
      const extra = document.createElement("p");
      extra.className = "modal-detail";
      extra.textContent = detail;
      box.appendChild(extra);
    }

    const actions = document.createElement("div");
    actions.className = "modal-actions";
    const cancel = document.createElement("button");
    cancel.type = "button";
    cancel.className = "btn-ghost";
    cancel.textContent = "Cancelar";
    const accept = document.createElement("button");
    accept.type = "button";
    accept.textContent = confirmLabel || "Continuar";
    if (danger) accept.className = "btn-danger";
    actions.appendChild(cancel);
    actions.appendChild(accept);
    box.appendChild(actions);
    overlay.appendChild(box);

    let done = false;
    const close = (value) => {
      if (done) return;
      done = true;
      document.removeEventListener("keydown", onKey, true);
      overlay.remove();
      resolve(value);
    };
    const onKey = (event) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        close(false);
        return;
      }
      // Foco atrapado: sin esto se podia tabular hasta los botones de debajo
      // del overlay y activarlos con Enter.
      if (event.key === "Tab") {
        const focusables = [cancel, accept];
        const activo = document.activeElement;
        const indice = focusables.indexOf(activo);
        event.preventDefault();
        const siguiente = event.shiftKey
          ? focusables[(indice <= 0 ? focusables.length : indice) - 1]
          : focusables[(indice + 1) % focusables.length];
        siguiente.focus();
      }
    };
    cancel.addEventListener("click", () => close(false));
    accept.addEventListener("click", () => close(true));
    overlay.addEventListener("click", (event) => {
      if (event.target === overlay) close(false);
    });
    document.addEventListener("keydown", onKey, true);

    document.body.appendChild(overlay);
    // Foco por defecto en Cancelar cuando la accion es destructiva.
    (danger ? cancel : accept).focus();
  });
}

let busy = false;

function setBusy(value) {
  busy = value;
  for (const button of document.querySelectorAll("main button")) {
    if (value) {
      // El snapshot se toma UNA sola vez: una segunda entrada en estado
      // ocupado guardaria "ya estaba deshabilitado" y los botones se quedarian
      // deshabilitados para siempre al restaurar.
      if (button.dataset.wasDisabled === undefined) {
        button.dataset.wasDisabled = button.disabled ? "1" : "0";
      }
      button.disabled = true;
    } else if (button.dataset.wasDisabled !== undefined) {
      button.disabled = button.dataset.wasDisabled === "1";
      delete button.dataset.wasDisabled;
    }
  }
  if (!value) refreshButtons();
}

// Pide confirmacion y ejecuta la operacion, una cada vez.
async function guardedRun(prompt, command, args) {
  if (busy) {
    appendLog("warn", "Ya hay una operacion en curso; espera a que termine.");
    return;
  }
  // Se marca ocupado ANTES de abrir el dialogo. El overlay solo tapa el raton:
  // con el teclado se podia llegar a otro boton y lanzar una segunda operacion
  // privilegiada mientras la primera esperaba confirmacion.
  setBusy(true);
  try {
    if (!(await confirmAction(prompt))) {
      appendLog("info", "Operacion cancelada. No se ha modificado nada.");
      return;
    }
    await runInstall(command, args);
  } catch {
    // runInstall ya registro el error en el log.
  } finally {
    setBusy(false);
  }
}

function badge(text, kind) {
  const el = document.createElement("span");
  el.className = "badge" + (kind ? ` ${kind}` : "");
  el.textContent = text;
  return el;
}

function render() {
  const env = catalog;
  const envText =
    `${env.distroName} · ${env.desktop} · ${env.session} · ${env.target || "sin target"}` +
    (env.compatible ? "" : " · NO COMPATIBLE");
  $("env").textContent = envText;

  if (env.distroId === "solus" || env.packageManager === "eopkg") {
    $("hyprland-card")?.classList.add("hidden");
    $("niri-card")?.classList.add("hidden");
    $("chaotic-card")?.classList.add("hidden");
  }


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
    // Un modulo ya instalado se marca: si se dejara sin marcar, "Actualizar"
    // reinstalaria el entorno sin el.
    box.checked = m.mandatory || Boolean(m.installedVersion);
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
  // Estos desinstaladores llaman a pacman/eopkg: necesitan la contrasena igual
  // que su instalador. Sin gatearlos fallaban siempre con "Introduce tu
  // contrasena de sudo en la interfaz".
  $("hyprland-uninstall").disabled = $("hyprland-pass").value.trim() === "";
  $("niri-uninstall").disabled = $("niri-pass").value.trim() === "";
  $("gaming-uninstall").disabled = $("gaming-pass").value.trim() === "";
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
    const previous = unlisten;
    unlisten = null;
    try {
      (await previous)();
    } catch {
      // El listener anterior ya no existe.
    }
  }
  try {
    // Se espera al registro del listener antes de invocar: si no, los primeros
    // mensajes del flujo se emiten antes de que haya nadie escuchando. Va
    // dentro del try para que un fallo al registrarlo tambien se reporte, en
    // vez de dejar la barra clavada en "Iniciando..." sin explicacion.
    unlisten = await listen("install://event", (event) => {
      const payload = event.payload;
      if (payload.kind === "log") {
        appendLog(payload.data.level, payload.data.message);
      } else if (payload.kind === "progress") {
        setProgress(payload.data.label, payload.data.percent);
      }
    });

    const result = await invoke(command, args);
    appendLog("ok", "Operacion completada.");
    setProgress("Completado", 100);
    // El refresco del catalogo va aparte: si fallara, la operacion ya se
    // completo y no debe reportarse como un fallo de la instalacion.
    try {
      catalog = await invoke("catalog_state");
      render();
      refreshUpdates();
    } catch (error) {
      appendLog("warn", `No se pudo refrescar el catalogo: ${error}`);
    }
    return result;
  } catch (error) {
    appendLog("err", String(error));
    setProgress("Fallo", 100);
    throw error;
  } finally {
    if (unlisten) {
      const fn = unlisten;
      unlisten = null;
      try {
        fn();
      } catch {
        // Nada que soltar.
      }
    }
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

  const INSTALL_NOTE =
    "Se instalaran paquetes del sistema con tu contrasena de sudo. Puedes seguir el detalle en el panel de Progreso.";

  $("kito-install").addEventListener("click", () => {
    const selection = {
      kitowall: $("mod-kitowall").checked,
      kilivepaper: $("mod-kilivepaper").checked,
      kisddm: $("mod-kisddm").checked,
    };
    const elegidos = Object.entries(selection)
      .filter(([, on]) => on)
      .map(([nombre]) => nombre);
    guardedRun(
      {
        title: "Instalar el entorno Kito",
        body: `Se instalaran KiUI y Kitsune Compositor${
          elegidos.length ? `, ademas de: ${elegidos.join(", ")}` : ""
        }.`,
        detail: INSTALL_NOTE,
        confirmLabel: "Instalar",
      },
      "install_kito",
      { selection, password: $("kito-pass").value }
    );
  });

  // Cada modal enumera TODAS las mutaciones del flujo, incluidas las preguntas
  // que el CLI haria por el camino y que la GUI acepta automaticamente.
  $("bauh-install").addEventListener("click", () => {
    const solus = catalog && (catalog.distroId === "solus" || catalog.packageManager === "eopkg");
    const pipxPkg = solus ? "pipx" : "python-pipx";
    // Mismo orden que install_bauh en flow.rs; en Solus no hay paso de pacman,
    // asi que la lista se numera al final para no dejar huecos.
    const pasos = [];
    if (!solus) {
      pasos.push(
        "Si tienes el paquete `bauh` de pacman, se DESINSTALARA con sudo para evitar conflictos " +
          "(este dialogo es la confirmacion)."
      );
    }
    pasos.push(`Se instalara el paquete ${pipxPkg} con sudo si falta.`);
    pasos.push(
      "Se descargara por HTTPS el ultimo release de github.com/The-Gekko/The-Gekko-Bauh " +
        "(manifiesto + .tar.zst) y se verificaran su tamano y su SHA-256."
    );
    pasos.push(
      "Se ejecutara `pipx install --force` sobre el arbol verificado: si ya habia un entorno pipx " +
        "de Bauh (bauh, gekko-bauh o bauh-fork-the-gekko) se reemplaza por el nuevo."
    );
    pasos.push(
      "Se escribiran ~/.local/share/applications/org.thegekko.bauh.desktop y el icono " +
        "~/.local/share/icons/hicolor/512x512/apps/org.thegekko.bauh.png."
    );
    guardedRun(
      {
        title: "Instalar la Tienda Bauh Fork",
        body: pasos.map((paso, indice) => `${indice + 1}) ${paso}`).join(" "),
        detail:
          "Hoy el ultimo release es v0.10.7 (se instala como distribucion `bauh`, lanzador ~/.local/bin/bauh); " +
          "desde v0.10.8-gekko.1 se instalara como `gekko-bauh` (lanzadores gekko-bauh, gekko-bauh-tray y gekko-bauh-cli). " +
          INSTALL_NOTE,
        confirmLabel: "Instalar",
      },
      "install_bauh",
      { password: $("bauh-pass").value }
    );
  });

  $("gekko-adb-install").addEventListener("click", () => {
    const solus = catalog && (catalog.distroId === "solus" || catalog.packageManager === "eopkg");
    const paquetes = solus
      ? "git python3 python-gobject libgtk-3 libgtk-4 android-tools scrcpy glib2 xdg-utils xdg-user-dirs curl (eopkg)"
      : "git python python-gobject gtk3 gtk4 android-tools android-udev scrcpy glib2 xdg-utils xdg-user-dirs curl (pacman)";
    guardedRun(
      {
        title: "Instalar Gekko ADB Studio",
        body:
          `1) Se instalaran con sudo los paquetes: ${paquetes}. ` +
          "2) Se clonara github.com/The-Gekko/gekko-adb (rama main, HEAD por HTTPS; el proyecto no publica releases, " +
          "asi que NO hay manifiesto ni SHA-256 que verificar) en ~/.cache/gekkoapp/gekko-adb. " +
          "3) Se ejecutara su install.sh --no-deps --assume-yes, que crea ~/.local/bin/gekko-adb, " +
          "~/.local/share/applications/com.gekko.adb.desktop, ~/.local/share/metainfo/com.gekko.adb.metainfo.xml, " +
          "~/.local/share/icons/hicolor/512x512/apps/gekko-adb.png y copia la app a ~/.local/share/gekko-adb/app " +
          "(la version anterior queda en app.bak.<fecha>). " +
          (solus
            ? ""
            : "4) Si existe /usr/lib/udev/rules.d/51-android.rules se ejecutara con sudo `udevadm control --reload-rules && udevadm trigger`. "),
        detail: INSTALL_NOTE,
        confirmLabel: "Instalar",
      },
      "install_gekko_adb",
      { password: $("gekko-adb-pass").value }
    );
  });

  $("gekkoapp-install").addEventListener("click", () => {
    guardedRun(
      {
        title: "Actualizar GekkoApp",
        body: "Se descargara el ultimo release verificado (manifiesto + SHA-256) de GekkoApp y se reemplazaran sus binarios en tu carpeta de usuario.",
        detail: "No requiere sudo. Tendras que reiniciar el Control Center al terminar.",
        confirmLabel: "Actualizar",
      },
      "install_gekkoapp",
      {}
    );
  });

  $("terminal-install").addEventListener("click", () => {
    guardedRun(
      {
        title: "Instalar Terminal Bonita",
        body:
          "Se instalaran ZSH, Starship, Kitty y sus plugins, y se sobrescribiran tres archivos tuyos: " +
          "~/.zshrc, ~/.config/starship.toml y ~/.config/fastfetch/config.jsonc. " +
          "Ademas se cambiara tu SHELL DE LOGIN a /bin/zsh usando sudo.",
        detail:
          "De cada archivo se guarda antes una copia con marca de tiempo junto al original. " +
          INSTALL_NOTE,
        confirmLabel: "Instalar",
      },
      "install_terminal",
      { password: $("terminal-pass").value }
    );
  });

  $("hyprland-install").addEventListener("click", () => {
    guardedRun(
      {
        title: "Instalar el preset Hyprland",
        body: "Se instalaran las herramientas del preset y se DESINSTALARAN dolphin, polkit-kde-agent y wofi si los tienes.",
        detail: INSTALL_NOTE,
        confirmLabel: "Instalar",
      },
      "install_hyprland",
      { password: $("hyprland-pass").value }
    );
  });

  $("niri-install").addEventListener("click", () => {
    guardedRun(
      {
        title: "Instalar el preset Niri",
        body: "Se instalaran las herramientas del preset y se DESINSTALARAN mako, swaybg, swayidle, swaylock y waybar si los tienes.",
        detail: INSTALL_NOTE,
        confirmLabel: "Instalar",
      },
      "install_niri",
      { password: $("niri-pass").value }
    );
  });

  $("gaming-install").addEventListener("click", () => {
    const gpu = $("gaming-gpu").value;
    guardedRun(
      {
        title: "Instalar el Gaming Setup",
        body: `Se instalara el driver Vulkan de ${gpu.toUpperCase()}, Steam y las utilidades gaming.`,
        detail: INSTALL_NOTE,
        confirmLabel: "Instalar",
      },
      "install_gaming_setup",
      { gpu, password: $("gaming-pass").value }
    );
  });

  $("chaotic-install").addEventListener("click", () => {
    guardedRun(
      {
        title: "Configurar Chaotic AUR",
        body: "Se anadira el repositorio Chaotic AUR a /etc/pacman.conf (con copia de seguridad) y se actualizara el sistema.",
        detail: INSTALL_NOTE,
        confirmLabel: "Configurar",
      },
      "install_chaotic_aur",
      { password: $("chaotic-pass").value }
    );
  });

  $("kito-uninstall")?.addEventListener("click", () => {
    guardedRun(
      {
        title: "Desinstalar el entorno Kito",
        body: "Se retiraran los modulos de Kito registrados por GekkoApp: sus lanzadores, entradas de menu, iconos y las versiones instaladas.",
        detail: "Solo se borra lo que GekkoApp instalo y tiene registrado.",
        confirmLabel: "Desinstalar",
        danger: true,
      },
      "uninstall_kito",
      {}
    );
  });

  $("bauh-uninstall")?.addEventListener("click", () => {
    guardedRun(
      {
        title: "Desinstalar la Tienda Bauh Fork",
        body:
          "1) `pipx uninstall` del entorno que GekkoApp registro al instalar (gekko-bauh, o bauh en el release v0.10.7) " +
          "y de los nombres antiguos (bauh-fork-the-gekko); un entorno `bauh` a secas solo se retira si GekkoApp lo instalo. " +
          "Con ello desaparecen los lanzadores de ~/.local/bin (gekko-bauh, gekko-bauh-tray, gekko-bauh-cli o bauh, bauh-tray, bauh-cli). " +
          "2) Se borran ~/.local/share/applications/org.thegekko.bauh.desktop y " +
          "~/.local/share/icons/hicolor/512x512/apps/org.thegekko.bauh.png. " +
          "3) Se elimina la copia verificada del release en ~/.local/lib/kitotsu/bauh-fork-the-gekko/ y su entrada en el estado de GekkoApp.",
        detail:
          "Se conservan ~/.config/gekko-bauh (y ~/.config/bauh si la tenias). Los paquetes del sistema instalados " +
          "con sudo no se desinstalan: python-pipx/pipx se queda. No requiere sudo.",
        confirmLabel: "Desinstalar",
        danger: true,
      },
      "uninstall_bauh",
      {}
    );
  });

  $("gekko-adb-uninstall")?.addEventListener("click", () => {
    guardedRun(
      {
        title: "Desinstalar Gekko ADB Studio",
        body:
          "Se borraran exactamente: ~/.local/bin/gekko-adb, ~/.local/share/applications/com.gekko.adb.desktop " +
          "(y los legacy GekkoADB.desktop y org.thegekko.gekko_adb.desktop), ~/.local/share/metainfo/com.gekko.adb.metainfo.xml, " +
          "~/.local/share/icons/hicolor/512x512/apps/gekko-adb.png, ~/.local/share/gekko-adb completo " +
          "(app, .env y las copias app.bak.*) y el clon ~/.cache/gekkoapp/gekko-adb.",
        detail:
          "Se conservan ~/.config/gekko-adb y ~/.local/state/gekko-adb/logs. Los paquetes del sistema instalados " +
          "con sudo (android-tools, scrcpy, gtk...) no se desinstalan. No requiere sudo.",
        confirmLabel: "Desinstalar",
        danger: true,
      },
      "uninstall_gekko_adb",
      {}
    );
  });

  $("terminal-uninstall")?.addEventListener("click", () => {
    guardedRun(
      {
        title: "Desinstalar Terminal Bonita",
        body: "Se ELIMINARA tu archivo ~/.zshrc.",
        detail: "Antes se guarda una copia con marca de tiempo junto al original. Los paquetes (zsh, starship, kitty...) se conservan.",
        confirmLabel: "Eliminar ~/.zshrc",
        danger: true,
      },
      "uninstall_terminal",
      {}
    );
  });

  $("hyprland-uninstall")?.addEventListener("click", () => {
    guardedRun(
      {
        title: "Desinstalar el preset Hyprland",
        body: "Se desinstalaran las herramientas propias del preset (nwg-look, xwayland-satellite).",
        detail: "Las dependencias de escritorio compartidas se conservan. Veras el plan exacto en el Progreso.",
        confirmLabel: "Desinstalar",
        danger: true,
      },
      "uninstall_hyprland",
      { password: $("hyprland-pass").value }
    );
  });

  $("niri-uninstall")?.addEventListener("click", () => {
    guardedRun(
      {
        title: "Desinstalar el preset Niri",
        body: "Se desinstalaran las herramientas propias del preset (nwg-look, xwayland-satellite, dconf-editor).",
        detail: "El compositor niri y las dependencias compartidas se conservan.",
        confirmLabel: "Desinstalar",
        danger: true,
      },
      "uninstall_niri",
      { password: $("niri-pass").value }
    );
  });

  $("gaming-uninstall")?.addEventListener("click", () => {
    guardedRun(
      {
        title: "Desinstalar el Gaming Setup",
        body: "Se desinstalaran gamemode, mangohud y los gestores de Proton.",
        detail: "Steam, Discord y Flatpak se conservan.",
        confirmLabel: "Desinstalar",
        danger: true,
      },
      "uninstall_gaming_setup",
      { password: $("gaming-pass").value }
    );
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
