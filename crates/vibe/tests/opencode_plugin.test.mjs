// Drives the generated OpenCode plugin with a fake `ctx`; run through the Rust test
// `opencode_plugin_node_test`, which sets VIBE_PLUGIN, VIBE_PLUGIN_RAW and VIBE_PLUGIN_BIN.
import test, { after } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";

const require = createRequire(import.meta.url);
const cp = require("node:child_process");
const spawned = [];
cp.spawn = (...args) => {
  const child = { unrefCalled: false, on() {}, unref() { this.unrefCalled = true; } };
  spawned.push({ args, child });
  return child;
};

const load = async (file) => (await import(pathToFileURL(file).href)).default;
const plugin = await load(process.env.VIBE_PLUGIN);
const rawPlugin = await load(process.env.VIBE_PLUGIN_RAW);
const BIN = process.env.VIBE_PLUGIN_BIN;

const tmpDirs = [];
after(() => {
  for (const dir of tmpDirs) fs.rmSync(dir, { recursive: true, force: true });
});

function tmpDir(prefix) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  tmpDirs.push(dir);
  return dir;
}

function storeDir(files = {}) {
  const dir = tmpDir("vibe-plugin-");
  for (const [name, text] of Object.entries(files)) fs.writeFileSync(path.join(dir, name), text);
  process.env.VIBE_STORE = dir;
  return dir;
}

function mcpFile(servers) {
  return { "mcp.json": JSON.stringify({ mcpServers: servers }) };
}

function fakeCtx(existing = {}) {
  const sets = {};
  const hooks = {};
  let transform = null;
  const ctx = {
    mcp: { transform(fn) { transform = fn; } },
    session: { hook(name, fn) { hooks[name] = fn; } },
  };
  return {
    ctx,
    sets,
    hooks,
    runTransform() {
      assert.ok(transform, "transform registered");
      transform({ get: (n) => existing[n], set: (n, v) => { sets[n] = v; } });
    },
    event() {
      const e = { system: [] };
      hooks.context(e);
      return e.system;
    },
  };
}

function withEnv(vars, fn) {
  const saved = {};
  for (const [k, v] of Object.entries(vars)) {
    saved[k] = process.env[k];
    process.env[k] = v;
  }
  try { return fn(); } finally {
    for (const [k, v] of Object.entries(saved)) {
      if (v === undefined) delete process.env[k]; else process.env[k] = v;
    }
  }
}

// The plugin branches on `process.platform`, so a test that means to exercise one branch must
// pin it: inheriting the host's platform makes a POSIX assertion mean something else on Windows.
// An omitted platform means "the host's" — pinning it to `undefined` would make every branch
// fall through to the default one and hide the caller's mistake.
function withPlatform(platform, fn) {
  if (platform === undefined) return fn();
  const saved = Object.getOwnPropertyDescriptor(process, "platform");
  Object.defineProperty(process, "platform", { value: platform });
  try { return fn(); } finally {
    Object.defineProperty(process, "platform", saved);
  }
}

test("module_default_export_has_id_and_setup", () => {
  assert.equal(plugin.id, "vibe-store");
  assert.equal(typeof plugin.setup, "function");
  assert.equal(Object.keys(plugin).sort().join(","), "id,setup");
});

test("registers_servers_set_if_absent", () => {
  storeDir(mcpFile({
    exa: { command: "exa-new" },
    serena: { command: "uvx", args: ["serena"], env: { A: "1" } },
  }));
  const f = fakeCtx({ exa: { type: "local", command: ["keep"] } });
  plugin.setup(f.ctx);
  f.runTransform();
  assert.deepEqual(Object.keys(f.sets), ["serena"]);
  assert.deepEqual(f.sets.serena, { type: "local", command: ["uvx", "serena"], environment: { A: "1" } });
});

test("remote_server_shape", () => {
  storeDir(mcpFile({
    ctx7: { type: "http", url: "https://c7.example/mcp", headers: { K: "v" } },
    bare: { type: "http", url: "https://b.example/mcp" },
  }));
  const f = fakeCtx();
  plugin.setup(f.ctx);
  f.runTransform();
  assert.deepEqual(f.sets.ctx7, { type: "remote", url: "https://c7.example/mcp", headers: { K: "v" } });
  assert.deepEqual(f.sets.bare, { type: "remote", url: "https://b.example/mcp" });
});

test("unset_variable_skips_server", () => {
  delete process.env.VIBE_T_UNSET;
  storeDir(mcpFile({
    a: { command: "x", args: ["${VIBE_T_UNSET}"] },
    b: { command: "x", env: { K: "${VIBE_T_UNSET}" } },
    c: { type: "http", url: "https://u/${VIBE_T_UNSET}" },
    d: { type: "http", url: "https://u", headers: { H: "${VIBE_T_UNSET}" } },
    ok: { command: "fine" },
  }));
  const f = fakeCtx();
  plugin.setup(f.ctx);
  f.runTransform();
  assert.deepEqual(Object.keys(f.sets), ["ok"]);
});

test("resolved_variables_in_headers_and_env", () => {
  withEnv({ VIBE_T_KEY: "s3cret", VIBE_T_BIN: "mybin" }, () => {
    storeDir(mcpFile({
      l: { command: "${VIBE_T_BIN}", args: ["--k", "${VIBE_T_KEY}"], env: { K: "x-${VIBE_T_KEY}" } },
      r: { type: "http", url: "https://h/${VIBE_T_KEY}", headers: { Authorization: "Bearer ${VIBE_T_KEY}" } },
    }));
    const f = fakeCtx();
    plugin.setup(f.ctx);
    f.runTransform();
    assert.deepEqual(f.sets.l, { type: "local", command: ["mybin", "--k", "s3cret"], environment: { K: "x-s3cret" } });
    assert.deepEqual(f.sets.r, { type: "remote", url: "https://h/s3cret", headers: { Authorization: "Bearer s3cret" } });
  });
});

test("disabled_maps_to_disabled_true", () => {
  storeDir(mcpFile({
    off: { command: "x", enabled: false },
    offr: { type: "http", url: "https://o", enabled: false },
    on: { command: "y", enabled: true },
  }));
  const f = fakeCtx();
  plugin.setup(f.ctx);
  f.runTransform();
  assert.equal(f.sets.off.disabled, true);
  assert.equal(f.sets.offr.disabled, true);
  assert.equal("disabled" in f.sets.on, false);
});

test("harness_allow_list_respected", () => {
  storeDir(mcpFile({
    only: { command: "a", harnesses: ["codex"] },
    both: { command: "b", harnesses: ["codex", "opencode"] },
    any: { command: "c" },
  }));
  const f = fakeCtx();
  plugin.setup(f.ctx);
  f.runTransform();
  assert.deepEqual(Object.keys(f.sets).sort(), ["any", "both"]);
});

test("context_hook_pushes_agents_text_per_event", () => {
  const dir = storeDir({ "AGENTS.md": "first rules\n" });
  const f = fakeCtx();
  plugin.setup(f.ctx);
  assert.deepEqual(f.event(), [{ type: "text", text: "first rules\n" }]);
  fs.writeFileSync(path.join(dir, "AGENTS.md"), "second rules\n");
  assert.deepEqual(f.event(), [{ type: "text", text: "second rules\n" }]);
});

test("missing_or_invalid_mcp_json_is_noop", () => {
  for (const files of [{}, { "mcp.json": "{broken" }, { "mcp.json": "[]" }, { "mcp.json": '{"mcpServers":[]}' }]) {
    storeDir(files);
    const f = fakeCtx();
    plugin.setup(f.ctx);
    f.runTransform();
    assert.deepEqual(f.sets, {});
  }
});

test("missing_agents_file_pushes_nothing", () => {
  for (const files of [{}, { "AGENTS.md": "  \n" }]) {
    storeDir(files);
    const f = fakeCtx();
    plugin.setup(f.ctx);
    assert.deepEqual(f.event(), []);
  }
});

test("store_dir_falls_back_to_xdg_then_home", () => withPlatform("linux", () => {
  const xdg = tmpDir("vibe-xdg-");
  fs.mkdirSync(path.join(xdg, "vibe"));
  fs.writeFileSync(path.join(xdg, "vibe", "AGENTS.md"), "xdg\n");
  const home = tmpDir("vibe-home-");
  fs.mkdirSync(path.join(home, ".config", "vibe"), { recursive: true });
  fs.writeFileSync(path.join(home, ".config", "vibe", "AGENTS.md"), "home\n");
  const saved = process.env.VIBE_STORE;
  delete process.env.VIBE_STORE;
  try {
    withEnv({ XDG_CONFIG_HOME: xdg, HOME: home }, () => {
      const f = fakeCtx();
      plugin.setup(f.ctx);
      assert.equal(f.event()[0].text, "xdg\n");
    });
    const savedXdg = process.env.XDG_CONFIG_HOME;
    delete process.env.XDG_CONFIG_HOME;
    try {
      withEnv({ HOME: home }, () => {
        const f = fakeCtx();
        plugin.setup(f.ctx);
        assert.equal(f.event()[0].text, "home\n");
      });
    } finally {
      if (savedXdg !== undefined) process.env.XDG_CONFIG_HOME = savedXdg;
    }
  } finally {
    if (saved === undefined) delete process.env.VIBE_STORE;
    else process.env.VIBE_STORE = saved;
  }
}));

test("spawns_session_start_detached_with_bin_constant", () => {
  storeDir({});
  spawned.length = 0;
  plugin.setup(fakeCtx().ctx);
  assert.equal(spawned.length, 1);
  const [bin, args, opts] = spawned[0].args;
  assert.equal(bin, BIN);
  assert.deepEqual(args, ["hook", "session-start", "--harness", "opencode"]);
  assert.deepEqual(opts, { detached: true, stdio: "ignore" });
  assert.equal(spawned[0].child.unrefCalled, true);
});

test("spawn_errors_are_swallowed", () => {
  storeDir({});
  const original = cp.spawn;
  cp.spawn = () => { throw new Error("boom"); };
  try {
    plugin.setup(fakeCtx().ctx);
  } finally {
    cp.spawn = original;
  }
});

test("spawn_skipped_without_bin_constant", () => {
  storeDir({});
  spawned.length = 0;
  const f = fakeCtx();
  rawPlugin.setup(f.ctx);
  assert.equal(spawned.length, 0);
  assert.equal(typeof f.hooks.context, "function");
});

test("plugin_never_writes_files", () => {
  const dir = storeDir({ "AGENTS.md": "x\n", ...mcpFile({ s: { command: "c" } }) });
  const before = fs.readdirSync(dir).sort();
  const writers = ["writeFileSync", "writeFile", "appendFileSync", "mkdirSync", "rmSync", "unlinkSync"];
  const saved = writers.map((n) => [n, fs[n]]);
  for (const n of writers) fs[n] = () => { throw new Error(`plugin called fs.${n}`); };
  try {
    const f = fakeCtx();
    plugin.setup(f.ctx);
    f.runTransform();
    f.event();
  } finally {
    for (const [n, fn] of saved) fs[n] = fn;
  }
  assert.deepEqual(fs.readdirSync(dir).sort(), before);
});

function runWith(vars, platform) {
  const keys = ["VIBE_STORE", "XDG_CONFIG_HOME", "APPDATA", "HOME", "USERPROFILE"];
  const saved = Object.fromEntries(keys.map((k) => [k, process.env[k]]));
  for (const k of keys) delete process.env[k];
  Object.assign(process.env, vars);
  try {
    return withPlatform(platform, () => {
      const f = fakeCtx();
      plugin.setup(f.ctx);
      f.runTransform();
      return Object.keys(f.sets);
    });
  } finally {
    for (const [k, v] of Object.entries(saved)) {
      if (v === undefined) delete process.env[k]; else process.env[k] = v;
    }
  }
}

function storeAt(root, rel, name) {
  const dir = path.join(root, rel);
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, "mcp.json"), JSON.stringify({ mcpServers: { [name]: { command: "x" } } }));
}

test("store_dir_matches_the_rust_resolution", () => {
  const home = tmpDir("vibe-plugin-home-");
  storeAt(home, ".config/vibe", "from_home");
  const xdg = tmpDir("vibe-plugin-xdg-");
  storeAt(xdg, "vibe", "from_xdg");
  const appdata = tmpDir("vibe-plugin-appdata-");
  storeAt(appdata, "vibe", "from_appdata");

  // Every POSIX assertion pins `linux`: the plugin branches on `process.platform`, so an
  // unpinned assertion would run the host's branch and mean something else on Windows.
  assert.deepEqual(runWith({ HOME: home }, "linux"), ["from_home"]);
  assert.deepEqual(runWith({ HOME: home, XDG_CONFIG_HOME: "relative/dir" }, "linux"), ["from_home"]);
  assert.deepEqual(runWith({ HOME: home, XDG_CONFIG_HOME: xdg }, "linux"), ["from_xdg"]);
  assert.deepEqual(
    runWith({ HOME: home, XDG_CONFIG_HOME: xdg, APPDATA: appdata }, "linux"),
    ["from_xdg"],
  );
  assert.deepEqual(
    runWith({ HOME: home, XDG_CONFIG_HOME: xdg, APPDATA: appdata }, "win32"),
    ["from_appdata"],
  );
  assert.deepEqual(runWith({ HOME: home, XDG_CONFIG_HOME: xdg }, "win32"), []);
});
