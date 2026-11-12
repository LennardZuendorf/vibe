// Task 3: real filesystem preservation, collision, retry and foreign ownership.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
const script = path.resolve('flow/scripts/migrate-discovery.sh');
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'vibe-discovery-'));
const put = (p,s) => { fs.mkdirSync(path.dirname(p),{recursive:true});fs.writeFileSync(p,s); };
const legacy = root => {
 const d=path.join(root,'.agents/skills/vibe');
 put(`${d}/SKILL.md`,'---\nname: vibe\n---\n');put(`${d}/state-machine.json`,'{}');put(`${d}/scripts/set-state.sh`,'# writer');
 put(`${d}/state.json`,'{"flow":"feature","phase":"impl","feature":"keep"}');
 put(`${d}/evidence/feature-keep.md`,'old receipt');put(`${d}/user-note.txt`,'user');return d;
};
const run = root => spawnSync('bash',[script,root],{encoding:'utf8'});
try {
 // Parser choice must not change the one-object/current-version admission rule.
 const executable=name=>{const found=spawnSync('/bin/bash',['-c','command -v "$1"','probe',name],{encoding:'utf8'});assert.equal(found.status,0,`missing ${name}`);return found.stdout.trim();};
 for(const parser of ['node','jq','neither']) {
  const bin=path.join(tmp,`parser-${parser}`);fs.mkdirSync(bin);
  for(const command of ['grep','mkdir','date','cat','rm','cmp','find','cp','mv'])fs.symlinkSync(executable(command),path.join(bin,command));
  if(parser!=='neither')fs.symlinkSync(parser==='node'?process.execPath:executable('jq'),path.join(bin,parser));
  for(const [label,bytes] of [['legacy','{}'],['v2','{"v":2}'],['multi-v2','{"v":2}\n{}'],['multi-legacy','{}\n{}'],['empty',''],['whitespace',' \n'],['array','[]'],['null','null'],['malformed','{broken']]) {
   const project=path.join(tmp,`${parser}-${label}`);const owned=legacy(project);fs.unlinkSync(`${owned}/state.json`);put(`${project}/.vibe/run/flow/state.json`,bytes);
   const result=spawnSync('/bin/bash',[script,project],{encoding:'utf8',env:{...process.env,PATH:bin},timeout:2000});
   assert.equal(result.error,undefined,`${parser}/${label}: ${result.error}`);
   const allowed=parser!=='neither'&&label==='legacy';
   assert.equal(result.status===0,allowed,`${parser}/${label}: ${result.stdout}${result.stderr}`);
   assert.equal(fs.readFileSync(`${project}/.vibe/run/flow/state.json`,'utf8'),bytes);
   assert.equal(fs.existsSync(owned),!allowed);assert.equal(fs.existsSync(`${project}/.agents/skills/flow`),allowed);
   if(!allowed)assert.equal(fs.existsSync(`${project}/.vibe/run/flow/evidence`),false,'failed parsing cannot copy receipts');
  }
 }
 console.log('PASS: Node/jq/no-parser canonical cursor admission matrix');
 // Shared runtime ownership must cover preservation reads and namespace mutation.
 const busy=path.join(tmp,'busy-lock');const busyOld=legacy(busy);
 const busyState=fs.readFileSync(`${busyOld}/state.json`);put(`${busy}/.vibe/run/flow/runtime.lock`,'foreign-holder');
 let blocked=run(busy);assert.notEqual(blocked.status,0,blocked.stdout);assert.match(blocked.stderr,/busy/);
 assert.equal(fs.existsSync(`${busy}/.agents/skills/flow`),false);assert.deepEqual(fs.readFileSync(`${busyOld}/state.json`),busyState);
 assert.equal(fs.readFileSync(`${busy}/.vibe/run/flow/runtime.lock`,'utf8'),'foreign-holder');assert.equal(fs.existsSync(`${busy}/.vibe/run/flow/state.json`),false);
 const fifo=path.join(tmp,'fifo-lock');const fifoOld=legacy(fifo);fs.mkdirSync(`${fifo}/.vibe/run/flow`,{recursive:true});
 assert.equal(spawnSync('mkfifo',[`${fifo}/.vibe/run/flow/runtime.lock`]).status,0);
 const fifoRun=await new Promise(resolve=>{
  const child=spawn('bash',[script,fifo],{detached:true,stdio:['ignore','pipe','pipe']});let stdout='',stderr='',timedOut=false;
  child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>stderr+=b);
  const timer=setTimeout(()=>{timedOut=true;process.kill(-child.pid,'SIGKILL');},1500);
  child.on('close',status=>{clearTimeout(timer);resolve({status,stdout,stderr,timedOut});});
 });
 assert.equal(fifoRun.timedOut,false,'existing FIFO lock must refuse without opening it');assert.notEqual(fifoRun.status,0);assert.match(fifoRun.stderr,/busy/);
 assert.equal(fs.lstatSync(`${fifo}/.vibe/run/flow/runtime.lock`).isFIFO(),true);assert.equal(fs.existsSync(fifoOld),true);assert.equal(fs.existsSync(`${fifo}/.agents/skills/flow`),false);
 for (const [label,current] of [['missing',null],['versionless','{"flow":"idle","phase":"idle"}'],['corrupt','{broken']]) {
  const adopted=path.join(tmp,`adopted-${label}`);const owned=legacy(adopted);put(`${adopted}/.vibe/run/flow/runs/run-1/revision-0.json`,'adopted');
  if(current!==null)put(`${adopted}/.vibe/run/flow/state.json`,current);
  blocked=run(adopted);assert.notEqual(blocked.status,0);assert.match(blocked.stderr,/adopted/);
  assert.equal(fs.existsSync(owned),true);assert.equal(fs.existsSync(`${adopted}/.agents/skills/flow`),false);
  assert.equal(fs.existsSync(`${adopted}/.vibe/run/flow/runtime.lock`),false,'release own lock');
  if(current===null)assert.equal(fs.existsSync(`${adopted}/.vibe/run/flow/state.json`),false);else assert.equal(fs.readFileSync(`${adopted}/.vibe/run/flow/state.json`,'utf8'),current);
 }
 const versioned=path.join(tmp,'versioned-without-records');const versionedOld=legacy(versioned);fs.unlinkSync(`${versionedOld}/state.json`);
 const versionedBytes=JSON.stringify({v:2,run_id:'run-1',revision:0,flow:'feature',phase:'plan',feature:'keep',updated:'2026-11-11T10:00:00Z',plan_digest:'plan',scope_digest:'scope',task_state:{},evidence_refs:[]});
 put(`${versioned}/.vibe/run/flow/state.json`,versionedBytes);blocked=run(versioned);assert.notEqual(blocked.status,0);assert.match(blocked.stderr,/versioned|adopted/);
 assert.equal(fs.readFileSync(`${versioned}/.vibe/run/flow/state.json`,'utf8'),versionedBytes);assert.equal(fs.existsSync(`${versioned}/.vibe/run/flow/evidence`),false);assert.equal(fs.existsSync(versionedOld),true);assert.equal(fs.existsSync(`${versioned}/.agents/skills/flow`),false);
 const root=path.join(tmp,'old'); const old=legacy(root);let r=run(root);assert.equal(r.status,0,r.stderr);
 assert.equal(fs.readFileSync(`${root}/.vibe/run/flow/state.json`,'utf8'),'{"flow":"feature","phase":"impl","feature":"keep"}');
 assert.equal(fs.readFileSync(`${root}/.vibe/run/flow/evidence/feature-keep.md`,'utf8'),'old receipt');
 assert.equal(fs.readFileSync(`${root}/.agents/skills/flow/user-note.txt`,'utf8'),'user');assert.equal(fs.existsSync(old),false);
 r=run(root);assert.equal(r.status,0,r.stderr); // interrupted/repeated upgrade
 const both=path.join(tmp,'both');legacy(both);put(`${both}/.agents/skills/flow/SKILL.md`,'new flow');r=run(both);assert.notEqual(r.status,0);assert.ok(r.stderr.includes('conflict'));assert.ok(fs.existsSync(`${both}/.agents/skills/vibe/user-note.txt`));
 const foreign=path.join(tmp,'foreign');put(`${foreign}/.agents/skills/vibe/SKILL.md`,'foreign');r=run(foreign);assert.equal(r.status,0);assert.equal(fs.readFileSync(`${foreign}/.agents/skills/vibe/SKILL.md`,'utf8'),'foreign');
 const newer=path.join(tmp,'new');put(`${newer}/.agents/skills/vibe/SKILL.md`,'---\nname: vibe\n---\n# Vibe toolkit');put(`${newer}/.agents/skills/flow/SKILL.md`,'flow');r=run(newer);assert.equal(r.status,0);
 const conflict=path.join(tmp,'runtime-conflict');legacy(conflict);put(`${conflict}/.vibe/run/flow/state.json`,'newer state');r=run(conflict);assert.notEqual(r.status,0);assert.equal(fs.readFileSync(`${conflict}/.vibe/run/flow/state.json`,'utf8'),'newer state');assert.ok(fs.existsSync(`${conflict}/.agents/skills/vibe/state.json`));
 console.log('PASS: legacy migration, resources, retry, both-generation/runtime conflicts and foreign skills');
} finally { fs.rmSync(tmp,{recursive:true,force:true}); }
// Installed copies must resolve their own Flow resources and uninstall only owned bytes.
const installed = fs.mkdtempSync(path.join(os.tmpdir(), 'vibe-installed-flow-'));
try {
 let r=spawnSync('bash',['install.sh',installed,'--platform','claude'],{encoding:'utf8'});assert.equal(r.status,0,r.stderr);
 for(const skill of ['vibe','flow','spec'])assert.ok(fs.existsSync(`${installed}/.agents/skills/${skill}/SKILL.md`));
 put(`${installed}/.agents/skills/flow/user-note.txt`,'keep user note');
 r=spawnSync('node',[`${installed}/.agents/skills/flow/engine/cli.mjs`,'orders','feature.impl'],{cwd:installed,env:{...process.env,CLAUDE_PROJECT_DIR:installed},encoding:'utf8'});assert.equal(r.status,0,r.stderr);assert.ok(r.stdout.startsWith('skill=flow'),r.stdout);
 r=spawnSync('bash',['install.sh',installed,'--uninstall'],{encoding:'utf8'});assert.equal(r.status,0,r.stderr);
 assert.equal(fs.readFileSync(`${installed}/.agents/skills/flow/user-note.txt`,'utf8'),'keep user note');
 assert.ok(fs.existsSync(`${installed}/.vibe/run/flow/state.json`),'preserve cursor without --yes');
 assert.equal(fs.existsSync(`${installed}/.agents/skills/flow/SKILL.md`),false);
 assert.equal(fs.existsSync(`${installed}/.agents/skills/vibe/SKILL.md`),false);
 console.log('PASS: installed Flow resources resolve; uninstall preserves cursor/user content');
} finally {fs.rmSync(installed,{recursive:true,force:true});}
// A standalone Spec upgrade must not repoint or hide the only legacy Flow entry.
const specOnly=fs.mkdtempSync(path.join(os.tmpdir(),'vibe-spec-only-upgrade-'));
try {
 const old=legacy(specOnly);const before=fs.readFileSync(`${old}/state.json`,'utf8');
 fs.mkdirSync(`${specOnly}/.claude/skills`,{recursive:true});fs.symlinkSync('../../.agents/skills/vibe',`${specOnly}/.claude/skills/vibe`);
 let r=spawnSync('bash',['install.sh',specOnly,'--only','spec','--platform','claude'],{encoding:'utf8'});assert.equal(r.status,0,r.stderr);
 assert.equal(fs.readFileSync(`${old}/state.json`,'utf8'),before);assert.ok(fs.existsSync(`${old}/state-machine.json`));assert.equal(fs.existsSync(`${specOnly}/.agents/skills/flow`),false);assert.ok(fs.existsSync(`${specOnly}/.agents/skills/spec/SKILL.md`));
 r=spawnSync('bash',['install.sh',specOnly,'--platform','claude'],{encoding:'utf8'});assert.equal(r.status,0,r.stderr);
 assert.equal(fs.readFileSync(`${specOnly}/.vibe/run/flow/state.json`,'utf8'),before);assert.ok(fs.existsSync(`${specOnly}/.claude/skills/flow/SKILL.md`));assert.ok(fs.readFileSync(`${old}/SKILL.md`,'utf8').includes('# Vibe toolkit'));
 console.log('PASS: Spec-only upgrade preserves legacy Flow; later full upgrade preserves state and registers both capabilities');
} finally {fs.rmSync(specOnly,{recursive:true,force:true});}
