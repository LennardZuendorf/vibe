// vibe-spec: literal-file
import { spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { mkdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const supervisor = fileURLToPath(new URL('./opencode-supervisor.mjs', import.meta.url));

export async function discover(command, home, { timeoutMs = 120000 } = {}) {
  if (process.platform === 'win32') throw new Error('OpenCode discovery requires POSIX process groups');
  home = path.resolve(home);
  const work = path.join(home, '.opencode-discovery');
  const cwd = path.join(work, 'cwd');
  const tmp = path.join(work, 'tmp');
  await Promise.all([mkdir(cwd, { recursive: true }), mkdir(tmp, { recursive: true })]);
  const env = {
    PATH: [path.dirname(process.execPath), '/usr/bin', '/bin', '/usr/sbin', '/sbin'].join(path.delimiter),
    HOME: home,
    XDG_CONFIG_HOME: path.join(home, '.config'),
    XDG_CACHE_HOME: path.join(home, '.cache'),
    XDG_DATA_HOME: path.join(home, '.local/share'),
    XDG_STATE_HOME: path.join(home, '.local/state'),
    TMPDIR: tmp,
    LANG: 'C.UTF-8',
    OPENCODE_PASSWORD: randomBytes(32).toString('base64url'),
    OPENCODE_DISABLE_MODELS_FETCH: '1',
    OPENCODE_DISABLE_AUTOUPDATE: '1',
    OPENCODE_DISABLE_PROJECT_CONFIG: '1',
    OPENCODE_DISABLE_CLAUDE_CODE: '1',
  };
  const deadline = Date.now() + timeoutMs;
  const children = [];
  function bounded(promise) {
    let timer;
    return Promise.race([
      promise,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error('OpenCode discovery timed out')), Math.max(0, deadline - Date.now()));
      }),
    ]).finally(() => clearTimeout(timer));
  }
  function launch(args) {
    const child = spawn(process.execPath, [supervisor, JSON.stringify([...command, ...args])], {
      cwd, env, detached: true, stdio: ['pipe', 'pipe', 'pipe', 'pipe'],
    });
    const entry = { child, stdout: '', stderr: '' };
    entry.closed = new Promise(resolve => child.once('close', resolve));
    child.stdout.on('data', data => { entry.stdout += data; });
    child.stderr.on('data', data => { entry.stderr += data; });
    const outputEnded = Promise.all([child.stdout, child.stderr].map(stream => new Promise(resolve => stream.once('end', resolve))));
    const result = new Promise((resolve, reject) => {
      child.once('error', reject);
      let control = '';
      child.stdio[3].on('data', data => {
        control += data;
        if (!control.includes('\n')) return;
        try { resolve(JSON.parse(control.slice(0, control.indexOf('\n')))); }
        catch (error) { reject(new Error(`Invalid OpenCode supervisor result: ${error.message}`)); }
      });
      child.once('close', () => reject(new Error('OpenCode supervisor exited before reporting its native result')));
    });
    entry.result = result.then(async ({ code, signal, error }) => {
      if (error) throw new Error(`OpenCode spawn failed: ${error}`);
      if (code !== 0) throw new Error(`OpenCode ${args.join(' ')} exited ${code ?? signal}: ${entry.stderr.trim()}`);
      await outputEnded;
      return entry.stdout;
    });
    // The retained server can exit before its result is awaited during cleanup.
    entry.result.catch(() => {});
    children.push(entry);
    return entry;
  }
  const run = args => bounded(launch(args).result);
  const parse = text => {
    try { return JSON.parse(text); }
    catch (error) { throw new Error(`Invalid OpenCode JSON: ${error.message}`); }
  };
  function signal(entry, name) {
    if (!entry.child.pid || entry.child.exitCode !== null || entry.child.signalCode !== null) return;
    try {
      process.kill(-entry.child.pid, name);
    } catch (error) { if (error.code !== 'ESRCH') throw error; }
  }
  try {
    const help = await run(['debug', '--help']);
    if (/^\s*(?:opencode\s+debug\s+)?skill(?:\s|$)/m.test(help)) {
      const skills = parse(await run(['debug', 'skill', '--pure']));
      if (!Array.isArray(skills) || skills.some(s => typeof s?.name !== 'string' || typeof s.location !== 'string')) {
        throw new Error('Invalid OpenCode skill list');
      }
      return skills;
    }
    if (!(await run(['api', '--help'])).includes('--standalone')) {
      throw new Error('OpenCode exposes no supported skill discovery route');
    }
    const server = launch(['serve', '--stdio', '--port', '0']);
    const line = await bounded(new Promise((resolve, reject) => {
      const onData = () => {
        const end = server.stdout.indexOf('\n');
        if (end !== -1) resolve(server.stdout.slice(0, end));
      };
      server.child.stdout.on('data', onData);
      server.result.then(() => reject(new Error('OpenCode server exited before readiness')), reject);
    }));
    const { url } = parse(line);
    const address = new URL(url);
    if (address.protocol !== 'http:' || address.hostname !== '127.0.0.1' || !address.port || address.username || address.password) {
      throw new Error('OpenCode readiness must name its private localhost server');
    }
    // v2 skill.list reads an initially empty registry without awaiting plugins.
    // model.list supplies the activation barrier on this SAME private server.
    parse(await run(['api', '--server', url, 'model.list']));
    const response = parse(await run(['api', '--server', url, 'skill.list']));
    if (!Array.isArray(response?.data) || response.data.some(s => typeof s?.name !== 'string' || typeof s.path !== 'string')) {
      throw new Error('Invalid OpenCode skill list');
    }
    return response.data.map(({ name, path: location }) => ({ name, location }));
  } finally {
    for (const entry of children) {
      entry.child.stdin.on('error', () => {});
      entry.child.stdin.end();
      signal(entry, 'SIGTERM');
    }
    await Promise.race([
      Promise.all(children.map(entry => entry.closed)),
      new Promise(resolve => setTimeout(resolve, 500)),
    ]);
    for (const entry of children) signal(entry, 'SIGKILL');
    await Promise.all(children.map(entry => entry.closed));
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  discover([process.argv[2]], process.argv[3]).then(
    skills => process.stdout.write(`${JSON.stringify(skills)}\n`),
    error => { console.error(error.message); process.exitCode = 1; },
  );
}
