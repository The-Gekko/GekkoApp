const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");

async function scenario({ accepted = true, missing = [], password = "", unavailable = false, prepareError = false, kitsune = false } = {}) {
  const elements = new Map();
  function element(id = "") {
    const e = { id, value: "", checked: false, disabled: false, dataset: {}, style: {},
      classList: { add() {}, remove() {}, toggle() {} }, handlers: {},
      appendChild() {}, replaceChildren() {}, setAttribute() {}, focus() {},
      addEventListener(event, fn) { this.handlers[event] = fn; } };
    return e;
  }
  const get = id => {
    if (!elements.has(id)) elements.set(id, element(id));
    return elements.get(id);
  };
  const calls = [];
  const plan = { planId: 7, components: ["KiUI 0.2.0", "Kitsune Compositor 0.1.3"],
    packages: ["qt6-base"], missingPackages: missing };
  const catalog = { distroName: "Arch", distroId: "arch", desktop: "niri", session: "wayland",
    compatible: true, kitoUnavailableReason: unavailable ? "Sesion incompatible" : null,
    items: [], kitoModules: [] };
  const context = vm.createContext({
    document: { getElementById: get, createElement: element, createTextNode: x => x,
      querySelectorAll: () => [], addEventListener() {} },
    window: { __TAURI__: { core: { invoke: async (cmd, args) => {
      calls.push({ cmd, args });
      if (cmd === "catalog_state") return catalog;
      if (cmd === "prepare_kito") {
        if (prepareError) throw new Error("Version incompatible");
        return plan;
      }
    } }, event: { listen: async () => () => {} } } }, console,
  });
  let source = fs.readFileSync("ui/app.js", "utf8");
  source = source.slice(0, source.lastIndexOf("init().catch"));
  vm.runInContext(source, context);
  vm.runInContext("loadTheme = async () => {}; refreshUpdates = async () => {}; render = () => {};", context);
  context.accepted = accepted;
  vm.runInContext("confirmAction = async prompt => { globalThis.reviewed = prompt; return accepted; }; runInstall = async (cmd, args) => invoke(cmd, args);", context);
  await vm.runInContext("init()", context);
  get("kito-pass").value = password;
  get("mod-kitsune").checked = kitsune;
  vm.runInContext("refreshButtons()", context);
  if (!unavailable) await get("kito-install").handlers.click();
  return { calls, reviewed: context.reviewed, disabled: get("kito-install").disabled };
}

test("base-only install prepares and confirms the exact plan without requiring sudo", async () => {
  const { calls, reviewed } = await scenario();
  assert.match(reviewed.body, /KiUI 0.2.0/);
  assert.deepEqual(JSON.parse(JSON.stringify(calls.find(c => c.cmd === "prepare_kito").args.selection)),
    { kitowall: false, kilivepaper: false, kisddm: false, kitsune: false });
  assert.equal(calls.find(c => c.cmd === "install_kito").args.planId, 7);
});

test("cancel and failed preflight never install", async () => {
  for (const options of [{ accepted: false }, { prepareError: true }, { missing: ["awww"] }]) {
    const { calls } = await scenario(options);
    assert.equal(calls.some(c => c.cmd === "install_kito"), false);
  }
});

test("missing dependencies can be confirmed with sudo password", async () => {
  const { calls, reviewed } = await scenario({ missing: ["awww"], password: "test-only" });
  assert.match(reviewed.detail, /awww/);
  assert.equal(calls.some(c => c.cmd === "install_kito"), true);
});

test("unsupported Kito environment disables only its install action", async () => {
  assert.equal((await scenario({ unavailable: true })).disabled, true);
});

test("Kitsune selection reaches the reviewed installation plan", async () => {
  const { calls } = await scenario({ kitsune: true });
  assert.equal(calls.find(c => c.cmd === "prepare_kito").args.selection.kitsune, true);
  assert.equal(calls.some(c => c.cmd === "install_kito"), true);
});
