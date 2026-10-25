// vibe:store-plugin v1
// Installed by `vibe`; rewritten on every sync. Reads the global store at runtime and never writes.
const VIBE_BIN = "__VIBE_BIN__";

const loadBuiltin = async (name) => {
  try {
    return await import(name);
  } catch {
    return null;
  }
};
const fs = await loadBuiltin("node:fs");
const childProcess = await loadBuiltin("node:child_process");
const nodePath = await loadBuiltin("node:path");

const PLACEHOLDER = /\$\{([A-Z_][A-Z0-9_]*)\}/g;

function isAbsolute(p) {
  return nodePath ? nodePath.isAbsolute(p) : p.startsWith("/");
}

// Mirrors the Rust resolution: VIBE_STORE, else %APPDATA%\vibe on Windows, else an absolute
// XDG_CONFIG_HOME, else $HOME/.config/vibe.
function storeDir() {
  const env = process.env;
  if (env.VIBE_STORE) return env.VIBE_STORE;
  if (process.platform === "win32") return env.APPDATA ? `${env.APPDATA}/vibe` : null;
  if (env.XDG_CONFIG_HOME && isAbsolute(env.XDG_CONFIG_HOME)) return `${env.XDG_CONFIG_HOME}/vibe`;
  return env.HOME ? `${env.HOME}/.config/vibe` : null;
}

function readStoreFile(name) {
  try {
    const dir = storeDir();
    return dir && fs ? fs.readFileSync(`${dir}/${name}`, "utf8") : null;
  } catch {
    return null;
  }
}

function expand(value) {
  if (typeof value !== "string") return null;
  let complete = true;
  const out = value.replace(PLACEHOLDER, (whole, name) => {
    const found = process.env[name];
    if (found === undefined) {
      complete = false;
      return whole;
    }
    return found;
  });
  return complete ? out : null;
}

function expandList(list) {
  if (list === undefined) return [];
  if (!Array.isArray(list)) return null;
  const out = list.map(expand);
  return out.includes(null) ? null : out;
}

function expandMap(map) {
  if (map === undefined) return {};
  if (!map || typeof map !== "object" || Array.isArray(map)) return null;
  const out = {};
  for (const [key, value] of Object.entries(map)) {
    const text = expand(value);
    if (text === null) return null;
    out[key] = text;
  }
  return out;
}

function translate(def) {
  if (!def || typeof def !== "object") return null;
  let out;
  if (def.type === "http") {
    const url = expand(def.url);
    const headers = expandMap(def.headers);
    if (url === null || headers === null) return null;
    out = { type: "remote", url };
    if (Object.keys(headers).length > 0) out.headers = headers;
  } else {
    const command = expand(def.command);
    const args = expandList(def.args);
    const environment = expandMap(def.env);
    if (command === null || args === null || environment === null) return null;
    out = { type: "local", command: [command, ...args] };
    if (Object.keys(environment).length > 0) out.environment = environment;
  }
  if (def.enabled === false) out.disabled = true;
  return out;
}

function registerServers(editor) {
  const text = readStoreFile("mcp.json");
  if (text === null) return;
  let servers;
  try {
    servers = JSON.parse(text).mcpServers;
  } catch {
    return;
  }
  if (!servers || typeof servers !== "object" || Array.isArray(servers)) return;
  for (const [name, def] of Object.entries(servers)) {
    if (def && Array.isArray(def.harnesses) && !def.harnesses.includes("opencode")) continue;
    if (editor.get(name) != null) continue;
    const server = translate(def);
    if (server !== null) editor.set(name, server);
  }
}

function pushInstructions(event) {
  const text = readStoreFile("AGENTS.md");
  if (text && text.trim() !== "" && Array.isArray(event?.system)) {
    event.system.push({ type: "text", text });
  }
}

function startSession() {
  if (!childProcess || !VIBE_BIN || VIBE_BIN.startsWith("__VIBE_")) return;
  try {
    const child = childProcess.default.spawn(
      VIBE_BIN,
      ["hook", "session-start", "--harness", "opencode"],
      { detached: true, stdio: "ignore" },
    );
    child.on?.("error", () => {});
    child.unref?.();
  } catch {
    // The session hook is best effort.
  }
}

export default {
  id: "vibe-store",
  setup(ctx) {
    ctx.mcp.transform(registerServers);
    ctx.session.hook("context", pushInstructions);
    startSession();
  },
};
