// vibe-spec: literal-file
import assert from 'node:assert/strict';
import { mkdtemp, readFile, realpath, writeFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { discover } from './opencode-discovery.mjs';

// No copied executables or host service: the double runs through this Node.
const fixture = String.raw`
import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
const [mode, ...args] = process.argv.slice(2);
const home = process.env.HOME;
const record = (value) => fs.appendFileSync(path.join(home, 'events'), JSON.stringify(value)+'\n');
record({args,pid:process.pid,cwd:process.cwd(),env:process.env});
if (args.join(' ') === 'debug --help') {
  console.log(mode === 'legacy-qualified' ? 'COMMANDS\n  opencode debug skill    List skills' : mode.startsWith('legacy') ? 'SUBCOMMANDS\n  skill    List skills' : 'SUBCOMMANDS\n  paths    Show paths\n  skills are discovered by the server');
} else if (args.join(' ') === 'api --help') {
  console.log('--standalone Run with a private server');
} else if (args.join(' ') === 'debug skill --pure') {
  const skills = ['spec','flow','vibe'].map(name => ({name,location:path.join(home,'.config/opencode/skills',name,'SKILL.md')}));
  if (mode === 'legacy-chunked') {
    skills[0].content = 'begin'+'x'.repeat(300000)+'end';
    const text = JSON.stringify(skills);
    for (let i=0;i<text.length;i+=4096) { process.stdout.write(text.slice(i,i+4096)); await new Promise(resolve=>setImmediate(resolve)); }
  } else console.log(JSON.stringify(skills));
} else if (args[0] === 'serve') {
  if (mode === 'server-exits-first' || mode === 'stubborn-descendant') {
    const child = spawn(process.execPath, ['-e', "process.on('SIGTERM',()=>{});process.send('ready');process.disconnect();setInterval(()=>{},1000)"], {stdio:['ignore','ignore','ignore','ipc']});
    child.once('message', () => {
      record({descendant:child.pid});
      if (mode === 'server-exits-first') process.exit(7);
      else console.log(JSON.stringify({url:'http://127.0.0.1:12345'}));
    });
    process.on('SIGTERM',()=>process.exit(0));
    process.stdin.resume();
    process.stdin.on('end',()=>process.exit(0));
  } else {
  const child = spawn(process.execPath, ['-e','setInterval(()=>{},1000)'], {stdio:'ignore'});
  record({descendant:child.pid});
  process.on('SIGTERM', () => { if (mode !== 'timeout') { child.kill(); child.once('close',()=>process.exit(0)); } });
  process.stdin.resume();
  process.stdin.on('end', () => { child.kill(); child.once('close',()=>process.exit(0)); });
  console.log(JSON.stringify({url:'http://127.0.0.1:12345'}));
  }
} else if (args[0] === 'api' && args[1] === '--server') {
  if (args[2] !== 'http://127.0.0.1:12345') process.exit(8);
  if (mode === 'timeout') setInterval(()=>{},1000);
  else if (args[3] === 'model.list') {
    if (mode === 'barrier-failure') process.exit(7);
    fs.writeFileSync(path.join(home,'activated'),'yes');
    console.log(JSON.stringify({location:{directory:process.cwd()},data:[]}));
  } else if (args[3] === 'skill.list') {
    if (!fs.existsSync(path.join(home,'activated'))) console.log(JSON.stringify({data:[]}));
    else if (mode === 'malformed') console.log('{bad json');
    else if (mode === 'invalid-list') console.log(JSON.stringify({data:[{name:'spec',path:42}]}));
    else console.log(JSON.stringify({location:{directory:process.cwd()},data:['spec','flow','vibe'].map(name=>({id:name,name,path:path.join(home,'.config/opencode/skills',name,'SKILL.md'),content:'skill'}))}));
  } else process.exit(9);
} else process.exit(10);
`;

async function setup(t, mode) {
  const home = await realpath(await mkdtemp(path.join(os.tmpdir(), 'vibe-opencode-double-')));
  const cli = path.join(home, 'cli.mjs');
  await writeFile(cli, fixture);
  t.after(async () => {
    // Also clean the deliberately reproduced survivor during RED runs.
    for (const event of await events(home).catch(() => [])) {
      if (!event.descendant) continue;
      try { process.kill(event.descendant, 'SIGKILL'); }
      catch (error) { if (error.code !== 'ESRCH') throw error; }
    }
    await rm(home, { recursive: true, force: true });
  });
  return { home, command: [process.execPath, cli, mode] };
}

async function events(home) {
  return (await readFile(path.join(home, 'events'), 'utf8')).trim().split('\n').map(JSON.parse);
}

function expected(home) {
  return ['spec', 'flow', 'vibe'].map(name => ({
    name, location: path.join(home, '.config/opencode/skills', name, 'SKILL.md'),
  }));
}

async function assertReaped(home) {
  for (const event of await events(home)) {
    const pid = event.pid ?? event.descendant;
    const deadline = Date.now() + 2000;
    while (true) {
      try { process.kill(pid, 0); }
      catch (error) {
        if (error.code === 'ESRCH') break;
        throw error;
      }
      assert.ok(Date.now() < deadline, `owned process ${pid} survived the 2000ms cleanup deadline`);
      await new Promise(resolve => setTimeout(resolve, 20));
    }
  }
}

for (const mode of ['legacy', 'legacy-qualified']) {
  test(`global-install/71: ${mode} capability keeps native debug discovery`, async t => {
    const { home, command } = await setup(t, mode);
    assert.deepEqual(await discover(command, home), expected(home));
    assert.equal((await events(home)).some(e => e.args?.[0] === 'serve'), false);
    await assertReaped(home);
  });
}

test('global-install/71: native command results retain complete multi-chunk output', async t => {
  const { home, command } = await setup(t, 'legacy-chunked');
  const skills = await discover(command, home);
  assert.deepEqual(skills.map(({ name, location }) => ({ name, location })), expected(home));
  assert.equal(skills[0].content, 'begin' + 'x'.repeat(300000) + 'end');
  await assertReaped(home);
});

test('global-install/71: v2 waits for activation on the retained server', async t => {
  const { home, command } = await setup(t, 'v2');
  assert.deepEqual(await discover(command, home), expected(home));
  const calls = (await events(home)).filter(e => e.args);
  assert.equal(calls.filter(e => e.args[0] === 'serve').length, 1);
  assert.deepEqual(calls.filter(e => e.args[0] === 'api' && e.args[1] === '--server').map(e => e.args[3]), ['model.list', 'skill.list']);
  await assertReaped(home);
});

test('global-install/71: cleanup does not signal completed command groups', async t => {
  const { home, command } = await setup(t, 'v2');
  const originalKill = process.kill;
  const signaled = [];
  process.kill = (pid, signal) => {
    signaled.push(pid);
    return originalKill(pid, signal);
  };
  try { assert.deepEqual(await discover(command, home), expected(home)); }
  finally { process.kill = originalKill; }
  for (const event of (await events(home)).filter(e => e.args && e.args[0] !== 'serve')) {
    assert.equal(signaled.includes(-event.pid), false, `completed command ${event.pid} was signaled`);
  }
  await assertReaped(home);
});

test('global-install/71: help and native operations exclude ambient credentials and config', async t => {
  const { home, command } = await setup(t, 'v2');
  const inherited = { OPENAI_API_KEY: 'sentinel-secret', CODEX_HOME: '/sentinel-codex', OPENCODE_CONFIG: '/sentinel-config', NODE_OPTIONS: '--no-warnings' };
  const previous = Object.fromEntries(Object.keys(inherited).map(k => [k, process.env[k]]));
  Object.assign(process.env, inherited);
  try { await discover(command, home); }
  finally {
    for (const [key, value] of Object.entries(previous)) {
      if (value === undefined) delete process.env[key]; else process.env[key] = value;
    }
  }
  for (const event of (await events(home)).filter(e => e.env)) {
    for (const key of Object.keys(inherited)) assert.equal(event.env[key], undefined, key);
    assert.equal(event.env.HOME, home);
    assert.equal(event.env.XDG_CONFIG_HOME, path.join(home, '.config'));
    assert.equal(event.env.XDG_CACHE_HOME, path.join(home, '.cache'));
    assert.equal(event.env.XDG_DATA_HOME, path.join(home, '.local/share'));
    assert.equal(event.env.XDG_STATE_HOME, path.join(home, '.local/state'));
    assert.equal(event.env.OPENCODE_DISABLE_MODELS_FETCH, '1');
    assert.equal(event.env.OPENCODE_DISABLE_AUTOUPDATE, '1');
    assert.ok(event.env.OPENCODE_PASSWORD.length >= 32);
    assert.ok(event.cwd.startsWith(home + path.sep));
  }
});

for (const [mode, message] of [['barrier-failure', /exited 7/], ['malformed', /JSON/], ['invalid-list', /Invalid OpenCode skill list/], ['timeout', /timed out/]]) {
  test(`global-install/71: ${mode} fails and reaps the private process group`, async t => {
    const { home, command } = await setup(t, mode);
    await assert.rejects(discover(command, home, { timeoutMs: mode === 'timeout' ? 3000 : 10000 }), message);
    assert.ok((await events(home)).some(e => e.descendant), 'cleanup control must start its private server');
    await assertReaped(home);
  });
}

test('global-install/71: server exits first without leaving its descendant', async t => {
  const { home, command } = await setup(t, 'server-exits-first');
  await assert.rejects(discover(command, home), /exited 7/);
  assert.ok((await events(home)).some(e => e.descendant));
  await assertReaped(home);
});

test('global-install/71: cleanup kills a TERM-ignoring descendant after server exit', async t => {
  const { home, command } = await setup(t, 'stubborn-descendant');
  assert.deepEqual(await discover(command, home), expected(home));
  assert.ok((await events(home)).some(e => e.descendant));
  await assertReaped(home);
});
