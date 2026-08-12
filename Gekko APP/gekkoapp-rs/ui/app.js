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
}

function refreshButtons() {
  $("kito-install").disabled = $("kito-pass").value.trim() === "";
  $("bauh-install").disabled = $("bauh-pass").value.trim() === "";
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

  $("kito-pass").addEventListener("input", refreshButtons);
  $("bauh-pass").addEventListener("input", refreshButtons);

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
}

init().catch((error) => {
  appendLog("err", `No se pudo iniciar el catalogo: ${error}`);
});
