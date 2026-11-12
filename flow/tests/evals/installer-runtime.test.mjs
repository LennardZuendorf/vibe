// Task 3: every installer cursor seed/removal shares runtime adoption and locking.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
const root=fs.mkdtempSync(path.join(os.tmpdir(),'vibe-installer-runtime-'));
const run=(...args)=>spawnSync('bash',['install.sh',root,'--platform','none',...args],{encoding:'utf8'});
try {
 let out=run();assert.equal(out.status,0,out.stderr);
 const runtime=path.join(root,'.vibe/run/flow'),cursor=path.join(runtime,'state.json');
 fs.mkdirSync(path.join(runtime,'runs'));fs.rmSync(cursor);
 out=run();assert.notEqual(out.status,0,'adopted missing cursor must require recovery');assert.equal(fs.existsSync(cursor),false,'no v1 resurrection');
 fs.rmdirSync(path.join(runtime,'runs'));fs.writeFileSync(path.join(runtime,'runtime.lock'),'other holder');
 out=run();assert.notEqual(out.status,0,'seed must respect busy runtime');assert.equal(fs.existsSync(cursor),false);assert.equal(fs.readFileSync(path.join(runtime,'runtime.lock'),'utf8'),'other holder');
 fs.rmSync(path.join(runtime,'runtime.lock'));const v2=JSON.stringify({v:2,flow:'feature',phase:'verify',feature:'test',run_id:'run-test',revision:3});fs.writeFileSync(cursor,v2);fs.mkdirSync(path.join(runtime,'runs'));
 out=run();assert.equal(out.status,0,out.stderr);assert.equal(fs.readFileSync(cursor,'utf8'),v2,'existing v2 preserved on resource reinstall');
 out=run('--uninstall','--yes');assert.notEqual(out.status,0,'v2 destructive cleanup needs coordinated recovery');assert.equal(fs.readFileSync(cursor,'utf8'),v2);
 console.log('PASS: installer seed/removal respects shared lock and adopted runtime; resource reinstall preserves existing v2');
} finally {fs.rmSync(root,{recursive:true,force:true});}
// Exercise the real installer under each parser availability and lock entry type.
const executable=name=>{const out=spawnSync('/bin/bash',['-c','command -v "$1"','probe',name],{encoding:'utf8'});assert.equal(out.status,0,`missing ${name}`);return out.stdout.trim();};
for(const parser of ['node','jq','neither']) {
 const project=fs.mkdtempSync(path.join(os.tmpdir(),`vibe-installer-${parser}-`));
 const runtime=path.join(project,'.vibe/run/flow');fs.mkdirSync(runtime,{recursive:true});
 const bin=path.join(project,'bin');fs.mkdirSync(bin);
 for(const command of ['bash','sh','mkdir','dirname','basename','cat','grep','rm','sed','head','date','mktemp','mv','find','cp','readlink','ln','chmod','env','awk','sort','cmp','diff','tr','rmdir','id'])fs.symlinkSync(executable(command),path.join(bin,command));
 if(parser!=='neither')fs.symlinkSync(parser==='node'?process.execPath:executable('jq'),path.join(bin,parser));
 const env={...process.env,PATH:bin};delete env.CLAUDE_PROJECT_DIR;
 try {
  for(const [label,bytes] of [['escaped','{"\\u0076":2}'],['versioned','{"v":2}'],['multi','{}\n{}'],['corrupt','{broken'],['array','[]'],['null','null'],['empty',''],['legacy','{}']]) {
   const cursor=path.join(runtime,'state.json');fs.writeFileSync(cursor,bytes);
   const result=spawnSync('/bin/bash',['install.sh',project,'--platform','none','--uninstall','--yes'],{encoding:'utf8',env,timeout:3000});
   assert.equal(result.error,undefined,`${parser}/${label}: ${result.error}`);
   const allowed=parser!=='neither'&&label==='legacy';assert.equal(result.status===0,allowed,`${parser}/${label}: ${result.stderr}`);
   if(!allowed)assert.equal(fs.readFileSync(cursor,'utf8'),bytes,'rejected metadata preserved');
  }
 } finally {fs.rmSync(project,{recursive:true,force:true});}
}
console.log('PASS: installer Node/jq/no-parser matrix preserves escaped-version/multidocument/corrupt/nonobject state');
const holders=fs.mkdtempSync(path.join(os.tmpdir(),'vibe-installer-holders-'));
try {
 const runtime=path.join(holders,'.vibe/run/flow');fs.mkdirSync(runtime,{recursive:true});const lock=path.join(runtime,'runtime.lock');
 const cursor=path.join(runtime,'state.json');fs.writeFileSync(cursor,'{}');
 for(const kind of ['regular','fifo','directory','dangling','symlink']) {
  if(kind==='regular')fs.writeFileSync(lock,'foreign-holder');
  else if(kind==='fifo')assert.equal(spawnSync('mkfifo',[lock]).status,0);
  else if(kind==='directory')fs.mkdirSync(lock);
  else {if(kind==='symlink')fs.writeFileSync(path.join(holders,'holder'),'foreign-holder');fs.symlinkSync(path.join(holders,kind==='dangling'?'absent':'holder'),lock);}
  for(const action of [[],['--uninstall','--yes']]) {
   const result=spawnSync('/bin/bash',['install.sh',holders,'--platform','none',...action],{encoding:'utf8',timeout:3000});
   assert.equal(result.error,undefined,`${kind}: ${result.error}`);assert.notEqual(result.status,0,`${kind}: refuse`);assert.ok(fs.lstatSync(lock),'holder retained');assert.equal(fs.readFileSync(cursor,'utf8'),'{}');
  }
  fs.rmSync(lock,{recursive:true,force:true});
 }
} finally {fs.rmSync(holders,{recursive:true,force:true});}
console.log('PASS: installer seed/removal refuse regular/FIFO/directory/dangling/symlink runtime locks without opening or deleting holders');
