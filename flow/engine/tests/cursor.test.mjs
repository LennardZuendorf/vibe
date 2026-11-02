// vibe-spec: literal-file
// engine/tests/cursor.test.mjs — engine/cursor.mjs (js-core/2, R6).
//
// The two load-bearing scenarios: an absent cursor reads as idle, and a
// present-but-unparseable cursor THROWS a named CursorParseError instead of
// silently degrading to idle — every bash cursor reader gets this wrong.
//
// readCursor/writeCursor take `vibeDir` directly (the directory state.json
// lives in), not a project root — see cursor.mjs's header. Callers resolve
// that directory via resolveVibeDir() in root.mjs; these tests either pass
// a bare fixture dir straight through (readCursor doesn't care how it was
// found) or, for the install-target scenario below, actually exercise
// resolveVibeDir() end to end.

import { mkdirSync, writeFileSync, copyFileSync, rmSync, readFileSync, symlinkSync, chmodSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { test, assert, assertEqual, makeSandbox, mkTempRoot } from './run.mjs';
import { spawnSync } from 'node:child_process';
import { readCursor, writeCursor, CursorParseError } from '../cursor.mjs';

const ROOT_MJS = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'root.mjs');
const JQ_PRESENT = spawnSync('bash', ['-c', 'command -v jq >/dev/null 2>&1']).status === 0;
const JQ_REQUIRED = 'validate-state: FAIL — jq is required but not installed.\n';
const SKILLS_NO_JQ = 'check-skills: WARN — jq not installed — cannot read delegates; assuming all external skills present.\n';
const DOCTOR_NO_JQ_CURSOR = 'ok   cursor flow cursor present (unverified — jq missing; validate-state.sh needs jq)';

function bareDir() {
  return mkTempRoot('vibe-cursor-');
}

test('readCursor: absent cursor file reads as idle', () => {
  const vibeDir = bareDir();
  try {
    const cursor = readCursor(vibeDir);
    assertEqual(cursor.state, 'idle');
    assertEqual(cursor.flow, 'idle');
    assertEqual(cursor.phase, 'idle');
    assertEqual(cursor.feature, null);
  } finally {
    rmSync(vibeDir, { recursive: true, force: true });
  }
});

test('readCursor: malformed (unparseable) cursor throws a named CursorParseError, not idle', () => {
  const vibeDir = bareDir();
  try {
    writeFileSync(path.join(vibeDir, 'state.json'), '{ this is not valid json');

    let threw = false;
    try {
      readCursor(vibeDir);
    } catch (err) {
      threw = true;
      assert(err instanceof CursorParseError, `expected CursorParseError, got ${err && err.name}`);
      assertEqual(err.name, 'CursorParseError');
    }
    assert(threw, 'expected readCursor to throw on a malformed cursor, not degrade to idle');
  } finally {
    rmSync(vibeDir, { recursive: true, force: true });
  }
});

test('readCursor: a JSON array at the top level is malformed, not idle', () => {
  const vibeDir = bareDir();
  try {
    writeFileSync(path.join(vibeDir, 'state.json'), '[1, 2, 3]');
    let threw = false;
    try {
      readCursor(vibeDir);
    } catch (err) {
      threw = true;
      assert(err instanceof CursorParseError);
    }
    assert(threw, 'a top-level JSON array is not a valid cursor shape');
  } finally {
    rmSync(vibeDir, { recursive: true, force: true });
  }
});

test('readCursor: a well-formed cursor reads flow/phase/feature and derives state', () => {
  const sandbox = makeSandbox({
    cursor: { flow: 'feature', phase: 'impl', feature: 'js-core', updated: '2026-01-01T00:00:00Z' },
  });
  try {
    const cursor = readCursor(sandbox.flowDir);
    assertEqual(cursor.flow, 'feature');
    assertEqual(cursor.phase, 'impl');
    assertEqual(cursor.feature, 'js-core');
    assertEqual(cursor.state, 'feature.impl');
  } finally {
    sandbox.cleanup();
  }
});

test('readCursor: single-token states collapse flow===phase into one key (idle, not idle.idle)', () => {
  const sandbox = makeSandbox();
  try {
    const cursor = readCursor(sandbox.flowDir);
    assertEqual(cursor.state, 'idle');
  } finally {
    sandbox.cleanup();
  }
});

test('writeCursor: writes atomically and readCursor reads back what was written', () => {
  const sandbox = makeSandbox();
  try {
    writeCursor(sandbox.flowDir, { flow: 'quick', phase: 'fix', feature: 'demo' });
    const cursor = readCursor(sandbox.flowDir);
    assertEqual(cursor.flow, 'quick');
    assertEqual(cursor.phase, 'fix');
    assertEqual(cursor.feature, 'demo');
    assertEqual(cursor.state, 'quick.fix');
  } finally {
    sandbox.cleanup();
  }
});

test('writeCursor: omitted feature defaults to null and matches jq key order (flow, phase, feature, updated)', () => {
  const sandbox = makeSandbox();
  try {
    writeCursor(sandbox.flowDir, { flow: 'idle', phase: 'idle' });
    const raw = readFileSync(sandbox.cursorPath, 'utf8');
    const keys = Object.keys(JSON.parse(raw));
    assertEqual(keys, ['flow', 'phase', 'feature', 'updated']);
    assert(raw.endsWith('}\n'), 'expected a trailing newline');
  } finally {
    sandbox.cleanup();
  }
});

test('sandbox self-check: makeSandbox\'s own cursor never triggers CursorParseError', () => {
  const sandbox = makeSandbox();
  try {
    readCursor(sandbox.flowDir); // must not throw
  } finally {
    sandbox.cleanup();
  }
});

// ---------------------------------------------------------------------------
// js-core/2 review, Finding 1 (CRITICAL) — install targets have no flow/ dir.
//
// readCursor(root) used to hardcode path.join(root, 'flow', 'state.json').
// That only ever worked in this dogfood repo, where .agents/skills/vibe is a
// symlink to flow/. A real install target (built by `cp -RL` into
// <TARGET>/.agents/skills/vibe/, per install.sh) has no flow/ directory at
// all — the old code would hit ENOENT and silently report "idle" while the
// real cursor said otherwise: exactly the wrong-answer class the
// CursorParseError fix above exists to prevent, just one layer up.
//
// This builds a fixture laid out exactly like a real install target
// (<tmp>/.agents/skills/vibe/{engine,state.json,state-machine.json}, no
// flow/, no .git, no .spec anywhere) and drives the *real* resolution path:
// a copy of root.mjs physically placed at the nested engine/ location (so
// its self-relative resolution triggers for real) resolves vibeDir, and the
// real (uncopied) cursor.mjs reads through it.
test('CRITICAL (Finding 1): cursor reads correctly from an install-target layout with no flow/ dir', async () => {
  const installRoot = mkTempRoot('vibe-install-cursor-');
  const vibeDir = path.join(installRoot, '.agents', 'skills', 'vibe');
  const engineDir = path.join(vibeDir, 'engine');
  mkdirSync(engineDir, { recursive: true });
  copyFileSync(ROOT_MJS, path.join(engineDir, 'root.mjs'));

  writeFileSync(
    path.join(vibeDir, 'state.json'),
    `${JSON.stringify(
      { flow: 'feature', phase: 'impl', feature: 'js-core', updated: '2026-08-11T18:18:41Z' },
      null,
      2,
    )}\n`,
  );

  // A cwd the resolver must NOT wander into via marker search or luck.
  const unrelatedCwd = mkTempRoot('vibe-install-cwd-');

  const prevEnv = process.env.CLAUDE_PROJECT_DIR;
  delete process.env.CLAUDE_PROJECT_DIR;

  try {
    assert(!existsFlowDir(installRoot), 'fixture must have no flow/ dir (this is the point)');

    const { resolveVibeDir } = await import(pathToFileURL(path.join(engineDir, 'root.mjs')).href);
    const resolved = resolveVibeDir({ cwd: unrelatedCwd });
    assertEqual(resolved, vibeDir);

    const cursor = readCursor(resolved);
    assertEqual(cursor.state, 'feature.impl');
    assertEqual(cursor.feature, 'js-core');
    assert(cursor.state !== 'idle', 'must not silently report idle for a real, present cursor');
  } finally {
    rmSync(installRoot, { recursive: true, force: true });
    rmSync(unrelatedCwd, { recursive: true, force: true });
    if (prevEnv !== undefined) process.env.CLAUDE_PROJECT_DIR = prevEnv;
  }
});

function existsFlowDir(root) {
  try {
    readFileSync(path.join(root, 'flow', 'state.json'));
    return true;
  } catch {
    return false;
  }
}

// flow-v2/1.1: real installed layout, both cursor generations.
test('runtime migration: JS prefers new cursor and preserves legacy bytes', () => {
  const sandbox = makeSandbox({ cursor: {flow: 'strategy', phase: 'spec', feature: 'old'} });
  try {
    const old = path.join(sandbox.flowDir, 'state.json');
    const before = readFileSync(old, 'utf8');
    const runtime = path.join(sandbox.dir, '.vibe/run/flow');
    mkdirSync(runtime, {recursive: true});
    writeFileSync(path.join(runtime, 'state.json'), JSON.stringify({flow:'quick', phase:'fix', feature:'new'}));
    assertEqual(readCursor(sandbox.flowDir).state, 'quick.fix');
    writeCursor(sandbox.flowDir, {flow:'quick',phase:'verify',feature:'new'});
    assertEqual(JSON.parse(readFileSync(path.join(runtime,'state.json'))).phase, 'verify');
    assertEqual(readFileSync(old,'utf8'), before);
  } finally { sandbox.cleanup(); }
});

// flow-v2/1.1: shell callers and JS agree before and after migration.
test('runtime migration: legacy setter and every shell reader prefer the new cursor', () => {
  const sb = makeSandbox({cursor:{flow:'strategy',phase:'spec',feature:'old'}});
  try {
    const scripts = path.join(sb.flowDir,'scripts'); mkdirSync(scripts);
    mkdirSync(path.join(sb.dir,'.agents/skills'),{recursive:true});
    symlinkSync(sb.flowDir,path.join(sb.dir,'.agents/skills/vibe'));
    copyFileSync(path.resolve(path.dirname(ROOT_MJS),'../SKILL.md'),path.join(sb.flowDir,'SKILL.md'));
    const source = path.resolve(path.dirname(ROOT_MJS),'../scripts');
    for (const name of ['runtime-state','set-state','detect-context','orders','validate-state','check-skills','doctor']) {
      copyFileSync(path.join(source,`${name}.sh`),path.join(scripts,`${name}.sh`));
    }
    const old = readFileSync(sb.cursorPath,'utf8');
    const runtime = path.join(sb.dir,'.vibe/run/flow'); mkdirSync(runtime,{recursive:true});
    writeFileSync(path.join(runtime,'state.json'),JSON.stringify({flow:'quick',phase:'fix',feature:'new',updated:null},null,2)+'\n');
    const result = spawnSync('bash',[path.join(scripts,'set-state.sh'),'quick.verify'],{encoding:'utf8',env:{...process.env,CLAUDE_PROJECT_DIR:sb.dir}});
    assertEqual(result.status,0,result.stderr);
    assertEqual(readCursor(sb.flowDir).state,'quick.verify');
    assertEqual(readCursor(sb.flowDir).feature,'new');
    assertEqual(readFileSync(sb.cursorPath,'utf8'),old);
    for (const reader of ['detect-context','orders','validate-state','check-skills','doctor']) {
      const out = spawnSync('bash',[path.join(scripts,`${reader}.sh`),...(reader === 'doctor' ? [sb.dir] : [])],{encoding:'utf8',env:{...process.env,CLAUDE_PROJECT_DIR:sb.dir}});
      if (reader === 'validate-state' && !JQ_PRESENT) {
        assertEqual(out.status,1); assertEqual(out.stdout,''); assertEqual(out.stderr,JQ_REQUIRED);
      } else if (reader === 'check-skills' && !JQ_PRESENT) {
        assertEqual(out.status,0); assertEqual(out.stdout,''); assertEqual(out.stderr,SKILLS_NO_JQ);
      } else if (reader === 'doctor' && !JQ_PRESENT) {
        assertEqual(out.status,0); assert(out.stdout.split('\n').includes(DOCTOR_NO_JQ_CURSOR),out.stdout);
        assert(!out.stdout.includes('no flow cursor (idle)'),out.stdout);
      } else {
        assertEqual(out.status,0,`${reader}: ${out.stderr}`);
        assert((reader === 'orders' || reader === 'check-skills') ? (out.stdout+out.stderr).includes('verification-before-completion') : out.stdout.includes('quick.verify'), `${reader}: ${out.stdout} ${out.stderr}`);
      }
    }
    rmSync(sb.cursorPath);
    assertEqual(readCursor(sb.flowDir).state,'quick.verify');
  } finally {sb.cleanup();}
});

test('runtime migration: JS and shell refuse escaping parent and leaf links', () => {
  for (const part of ['.vibe','.vibe/run','.vibe/run/flow','.vibe/run/flow/state.json']) {
    const sb=makeSandbox(); const outside=bareDir();
    try {
      const target=path.join(sb.dir,part); mkdirSync(path.dirname(target),{recursive:true});
      const destination=part.endsWith('state.json') ? path.join(outside,'keeper') : outside;
      if(part.endsWith('state.json')) writeFileSync(destination,'{"flow":"quick","phase":"fix"}');
      else {mkdirSync(path.join(outside, part==='.vibe' ? 'run/flow' : part==='.vibe/run' ? 'flow' : ''),{recursive:true});
        writeFileSync(path.join(outside,part==='.vibe'?'run/flow/state.json':part==='.vibe/run'?'flow/state.json':'state.json'),'{"flow":"quick","phase":"fix"}');}
      symlinkSync(destination,target);
      const protectedPath=path.join(sb.dir,'.vibe/run/flow/state.json');
      const before=readFileSync(protectedPath,'utf8');
      let refused=false; try {writeCursor(sb.flowDir,{flow:'idle',phase:'idle'});} catch {refused=true;}
      assert(refused,`JS accepted ${part}`);
      const source=path.resolve(path.dirname(ROOT_MJS),'../scripts'); const scripts=path.join(sb.flowDir,'scripts');mkdirSync(scripts);
      for(const name of ['runtime-state','set-state'])copyFileSync(path.join(source,`${name}.sh`),path.join(scripts,`${name}.sh`));
      const out=spawnSync('bash',[path.join(scripts,'set-state.sh'),'idle'],{encoding:'utf8',env:{...process.env,CLAUDE_PROJECT_DIR:sb.dir}});
      assertEqual(out.status,1,`shell accepted ${part}: ${out.stderr}`);
      assertEqual(readFileSync(protectedPath,'utf8'),before,`outside bytes changed: ${part}`);
    } finally {sb.cleanup();rmSync(outside,{recursive:true,force:true});}
  }
});

// flow-v2/1.1: mv must not interpret a cursor directory as its destination home.
test('runtime migration: shell refuses a directory cursor or internal directory link', () => {
  for (const linked of [false,true]) {
    const sb=makeSandbox();
    try {
      const runtime=path.join(sb.dir,'.vibe/run/flow');mkdirSync(runtime,{recursive:true});
      const target=path.join(runtime,'state.json');
      if(linked){mkdirSync(path.join(sb.dir,'keeper'));symlinkSync(path.join(sb.dir,'keeper'),target);}
      else mkdirSync(target);
      const scripts=path.join(sb.flowDir,'scripts');mkdirSync(scripts);
      const source=path.resolve(path.dirname(ROOT_MJS),'../scripts');
      for(const name of ['runtime-state','set-state'])copyFileSync(path.join(source,`${name}.sh`),path.join(scripts,`${name}.sh`));
      const out=spawnSync('bash',[path.join(scripts,'set-state.sh'),'idle'],{encoding:'utf8',env:{...process.env,CLAUDE_PROJECT_DIR:sb.dir}});
      assertEqual(out.status,1,`directory cursor accepted: ${out.stderr}`);
    } finally { sb.cleanup();}
  }
});

// flow-v2/1.1: physical confinement must retain safe in-project links.
test('runtime migration: JS and shell allow internal parent and regular leaf links', () => {
  const sb=makeSandbox();
  try {
    mkdirSync(path.join(sb.dir,'runtime/run/flow'),{recursive:true});
    symlinkSync('runtime',path.join(sb.dir,'.vibe'));
    const keeper=path.join(sb.dir,'keeper.json');
    const before='{"flow":"quick","phase":"fix","feature":"inside"}';
    writeFileSync(keeper,before);
    symlinkSync(keeper,path.join(sb.dir,'.vibe/run/flow/state.json'));
    writeCursor(sb.flowDir,{flow:'quick',phase:'verify',feature:'inside'});
    assertEqual(readCursor(sb.flowDir).state,'quick.verify');
    assertEqual(readFileSync(keeper,'utf8'),before);
    const scripts=path.join(sb.flowDir,'scripts');mkdirSync(scripts);
    const source=path.resolve(path.dirname(ROOT_MJS),'../scripts');
    for(const name of ['runtime-state','set-state'])copyFileSync(path.join(source,`${name}.sh`),path.join(scripts,`${name}.sh`));
    const out=spawnSync('bash',[path.join(scripts,'set-state.sh'),'idle'],{encoding:'utf8',env:{...process.env,CLAUDE_PROJECT_DIR:sb.dir}});
    assertEqual(out.status,0,out.stderr);
    assertEqual(readCursor(sb.flowDir).state,'idle');
    assertEqual(readFileSync(keeper,'utf8'),before);
  } finally {sb.cleanup();}
});

// flow-v2/1.1: malformed new entries are authoritative, preserving legacy bytes.
test('runtime migration: dangling, directory and unreadable new cursors report invalid', () => {
  for (const kind of ['dangling','directory','unreadable','inaccessible-parent']) {
    const sb = makeSandbox({cursor:{flow:'quick',phase:'fix',feature:'old'}});
    try {
      const scripts = path.join(sb.flowDir,'scripts'); mkdirSync(scripts);
    mkdirSync(path.join(sb.dir,'.agents/skills'),{recursive:true});
    symlinkSync(sb.flowDir,path.join(sb.dir,'.agents/skills/vibe'));
    copyFileSync(path.resolve(path.dirname(ROOT_MJS),'../SKILL.md'),path.join(sb.flowDir,'SKILL.md'));
      const source = path.resolve(path.dirname(ROOT_MJS),'../scripts');
      for (const name of ['runtime-state','detect-context','orders','validate-state','check-skills','doctor']) {
        copyFileSync(path.join(source,`${name}.sh`),path.join(scripts,`${name}.sh`));
      }
      const before = readFileSync(sb.cursorPath,'utf8');
      const current = path.join(sb.dir,'.vibe/run/flow/state.json'); mkdirSync(path.dirname(current),{recursive:true});
      if (kind === 'dangling') symlinkSync('missing',current);
      else if (kind === 'directory') mkdirSync(current);
      else if (kind === 'unreadable') { writeFileSync(current,'{}'); chmodSync(current,0); }
      else { writeFileSync(current,'{}'); chmodSync(path.dirname(current),0); }
      let error; try {readCursor(sb.flowDir);} catch (err) {error=err;}
      assert(error instanceof CursorParseError, `${kind}: JS must report malformed`);
      for (const reader of ['detect-context','orders','validate-state','check-skills','doctor']) {
        const args = reader === 'doctor' ? [sb.dir] : [];
        const out=spawnSync('bash',[path.join(scripts,`${reader}.sh`),...args],{encoding:'utf8',env:{...process.env,CLAUDE_PROJECT_DIR:sb.dir}});
        assertEqual(out.status,reader === 'validate-state' ? 1 : 0, `${kind}/${reader}: ${out.stderr}`);
        assert((out.stderr+out.stdout).includes('invalid'),`${kind}/${reader}: must report invalid: ${out.stdout} ${out.stderr}`);
        assert(!out.stdout.includes('quick.fix'),`${kind}/${reader}: no old state`);
      }
      const fallback=spawnSync('bash',[path.resolve(path.dirname(ROOT_MJS),'../hooks-fallback/stop-gate.sh')],{encoding:'utf8',input:'{}',env:{...process.env,CLAUDE_PROJECT_DIR:sb.dir}});
      assertEqual(fallback.status,0,fallback.stderr);
      assert(fallback.stderr.includes('invalid'),`${kind}/Stop: ${fallback.stderr}`);
      assertEqual(readFileSync(sb.cursorPath,'utf8'),before);
    } finally { if (kind === 'inaccessible-parent') chmodSync(path.join(sb.dir,'.vibe/run/flow'),0o700); sb.cleanup();}
  }
});

// flow-v2/F6 review2: absence requires a valid/searchable full ancestor chain.
for (const ancestor of ['.vibe','.vibe/run','.vibe/run/flow']) {
  for (const kind of ['inaccessible','non-directory','dangling','looping','absent','search-only','search-only-absent']) {
    test(`cursor ancestry: ${ancestor} ${kind} across JS and every shell reader`, () => {
      const sb=makeSandbox({cursor:{flow:'quick',phase:'fix',feature:'old',updated:'2026-01-01T00:00:00Z'}});
      const target=path.join(sb.dir,ancestor);
      try {
        mkdirSync(path.join(sb.dir,'.agents/skills'),{recursive:true});
        symlinkSync(sb.flowDir,path.join(sb.dir,'.agents/skills/vibe'));
        copyFileSync(path.resolve(path.dirname(ROOT_MJS),'../SKILL.md'),path.join(sb.flowDir,'SKILL.md'));
        const scripts=path.join(sb.flowDir,'scripts');mkdirSync(scripts);
        for (const name of ['runtime-state','detect-context','orders','validate-state','check-skills','doctor']) {
          copyFileSync(path.resolve(path.dirname(ROOT_MJS),`../scripts/${name}.sh`),path.join(scripts,`${name}.sh`));
        }
        const before=readFileSync(sb.cursorPath,'utf8');
        const current=path.join(sb.dir,'.vibe/run/flow/state.json');mkdirSync(path.dirname(current),{recursive:true});
        writeFileSync(current,JSON.stringify({flow:'quick',phase:'verify',feature:'new',updated:'2026-01-01T00:00:00Z'}));
        if (kind === 'inaccessible') chmodSync(target,0);
        else if (kind === 'search-only' || kind === 'search-only-absent') { if(kind === 'search-only-absent') rmSync(current); chmodSync(target,0o111); }
        else {
          rmSync(target,{recursive:true});
          if (kind === 'non-directory') writeFileSync(target,'not a directory');
          if (kind === 'dangling') symlinkSync('missing-target',target);
          if (kind === 'looping') symlinkSync(path.basename(target),target);
        }
        const valid=kind === 'absent' || kind === 'search-only' || kind === 'search-only-absent';
        const expected=kind.endsWith('absent') ? 'quick.fix' : 'quick.verify';
        if (valid) assertEqual(readCursor(sb.flowDir).state,expected);
        else { let err;try {readCursor(sb.flowDir);}catch(e){err=e;}assert(err instanceof CursorParseError,`${kind}: JS must report malformed`); }
        for (const reader of ['detect-context','orders','validate-state','check-skills','doctor']) {
          const args=reader === 'doctor' ? [sb.dir] : [];
          const out=spawnSync('bash',[path.join(scripts,`${reader}.sh`),...args],{encoding:'utf8',env:{...process.env,CLAUDE_PROJECT_DIR:sb.dir}});
          assertEqual(out.status, reader === 'validate-state' && (!valid || !JQ_PRESENT) ? 1 : 0,`${ancestor}/${kind}/${reader}: ${out.stderr}`);
          const text=out.stdout+out.stderr;
          if (valid) {
            assert(!text.includes('invalid'),`${ancestor}/${kind}/${reader}: ${text}`);
            if (reader === 'validate-state' && !JQ_PRESENT) {
              assertEqual(out.stdout,''); assertEqual(out.stderr,JQ_REQUIRED);
            } else if (reader === 'check-skills' && !JQ_PRESENT) {
              assertEqual(out.stdout,''); assertEqual(out.stderr,SKILLS_NO_JQ);
            } else if (reader === 'doctor' && !JQ_PRESENT) {
              assert(out.stdout.split('\n').includes(DOCTOR_NO_JQ_CURSOR),out.stdout);
              assert(!out.stdout.includes('no flow cursor (idle)'),out.stdout);
            } else assert((reader === 'orders' || reader === 'check-skills') ? text.includes(kind.endsWith('absent') ? 'test-driven-development' : 'verification-before-completion') : text.includes(expected),`${ancestor}/${kind}/${reader}: ${text}`);
          } else {
            assert(text.includes('invalid'),`${ancestor}/${kind}/${reader}: no invalid report: ${text}`);
            assert(!out.stdout.includes('quick.fix'),`${ancestor}/${kind}/${reader}: used old state`);
            if(reader === 'doctor') assert(!out.stdout.includes('no flow cursor (idle)'),`${ancestor}/${kind}: healthy absence reported`);
          }
        }
        const out=spawnSync('bash',[path.resolve(path.dirname(ROOT_MJS),'../hooks-fallback/stop-gate.sh')],{encoding:'utf8',input:'{}',env:{...process.env,CLAUDE_PROJECT_DIR:sb.dir}});
        assertEqual(out.status,kind === 'search-only' ? 2 : 0,out.stderr);
        if(valid) assert(!out.stderr.includes('invalid'),out.stderr);
        else assert(out.stderr.includes('invalid') && !out.stderr.includes('still in quick.fix'),out.stderr);
        const selected=spawnSync('/bin/bash',['-c','source "$1"; runtime_state_path "$2"','cursor-selector',path.join(scripts,'runtime-state.sh'),sb.flowDir],{encoding:'utf8',env:{PATH:'/nonexistent',CLAUDE_PROJECT_DIR:sb.dir}});
        assertEqual(selected.status,0,selected.stderr);
        assertEqual(selected.stdout.trimEnd(),kind.endsWith('absent') ? sb.cursorPath : current);
        assertEqual(selected.stderr.includes('invalid'),!valid);
        assertEqual(readFileSync(sb.cursorPath,'utf8'),before);
      } finally {
        if(kind === 'inaccessible' || kind === 'search-only' || kind === 'search-only-absent') chmodSync(target,0o700);
        sb.cleanup();
      }
    });
  }
}

test('cursor ancestry: standalone fallback classifier stays identical to the shared helper', () => {
  const source=path.resolve(path.dirname(ROOT_MJS),'..');
  const body=p=>readFileSync(p,'utf8').match(/^runtime_cursor_status\(\) \{[\s\S]*?^\}/m)?.[0];
  const helper=body(path.join(source,'scripts/runtime-state.sh'));
  assert(helper,'classifier population floor');
  assertEqual(body(path.join(source,'hooks-fallback/stop-gate.sh')),helper);
});

for (const kind of ['inaccessible','search-only']) {
  test(`cursor ancestry: project root ${kind} from externally located readers`, () => {
    const sb=makeSandbox({cursor:{flow:'quick',phase:'fix',feature:'old',updated:'2026-01-01T00:00:00Z'}});
    const before=readFileSync(sb.cursorPath,'utf8');
    try {
      const current=path.join(sb.dir,'.vibe/run/flow/state.json');mkdirSync(path.dirname(current),{recursive:true});
      writeFileSync(current,JSON.stringify({flow:'quick',phase:'verify',feature:'new',updated:'2026-01-01T00:00:00Z'}));
      chmodSync(sb.dir,kind === 'inaccessible' ? 0 : 0o111);
      if(kind === 'inaccessible') {
        let err;try {readCursor(sb.flowDir);}catch(e){err=e;}assert(err instanceof CursorParseError);
        for(const reader of ['detect-context','orders','validate-state','check-skills','doctor']) {
          const script=path.resolve(path.dirname(ROOT_MJS),`../scripts/${reader}.sh`);
          const out=spawnSync('bash',[script,...(reader === 'doctor' ? [sb.dir] : [])],{encoding:'utf8',env:{...process.env,CLAUDE_PROJECT_DIR:sb.dir}});
          assertEqual(out.status,reader === 'validate-state' ? 1 : 0, out.stderr);
          assert((out.stdout+out.stderr).includes('invalid'),`${reader}: ${out.stdout} ${out.stderr}`);
          assert(!out.stdout.includes('no flow cursor (idle)'),out.stdout);
        }
        const stop=spawnSync('bash',[path.resolve(path.dirname(ROOT_MJS),'../hooks-fallback/stop-gate.sh')],{encoding:'utf8',input:'{}',env:{...process.env,CLAUDE_PROJECT_DIR:sb.dir}});
        assertEqual(stop.status,0);assert(stop.stderr.includes('invalid'),stop.stderr);
      } else assertEqual(readCursor(sb.flowDir).state,'quick.verify');
      const selected=spawnSync('/bin/bash',['-c','source "$1"; runtime_state_path "$2"','root-selector',path.resolve(path.dirname(ROOT_MJS),'../scripts/runtime-state.sh'),sb.flowDir],{encoding:'utf8',env:{PATH:'/nonexistent',CLAUDE_PROJECT_DIR:sb.dir}});
      assertEqual(selected.status,0);assertEqual(selected.stdout.trimEnd(),current);
      assertEqual(selected.stderr.includes('invalid'),kind === 'inaccessible');
    } finally {chmodSync(sb.dir,0o700);assertEqual(readFileSync(sb.cursorPath,'utf8'),before);sb.cleanup();}
  });
}
