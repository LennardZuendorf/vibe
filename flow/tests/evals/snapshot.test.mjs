// Task 3: verify the actual uncommitted source snapshot in a disposable clone.
// No commit is created. This proves package/install bytes, not marketplace fetch.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';
const source=process.cwd();const work=fs.mkdtempSync(path.join(os.tmpdir(),'vibe-source-snapshot-'));
const clone=path.join(work,'clone');const installed=path.join(work,'installed');
function run(command,args,cwd=source,env=process.env) {const r=spawnSync(command,args,{cwd,env,encoding:'utf8',maxBuffer:8*1024*1024});assert.equal(r.status,0,`${command} ${args.join(' ')}\n${r.stdout}\n${r.stderr}`);return r.stdout;}
function value(root,p) {
 const full=path.join(root,p);let stat;try{stat=fs.lstatSync(full);}catch(e){if(e.code==='ENOENT')return ['absent'];throw e;}
 if(stat.isSymbolicLink())return ['link',fs.readlinkSync(full)];
 return ['file',stat.mode&0o777,crypto.createHash('sha256').update(fs.readFileSync(full)).digest('hex')];
}
try {
 const base=run('git',['rev-parse','HEAD']).trim();run('git',['clone','-q','--no-hardlinks',source,clone]);assert.equal(run('git',['rev-parse','HEAD'],clone).trim(),base);
 const files=[...new Set(run('git',['ls-files','--cached','--others','--exclude-standard','-z']).split('\0').filter(Boolean))].sort();
 const entries=[];
 for(const p of files) {
  const from=path.join(source,p),to=path.join(clone,p),entry=value(source,p);entries.push([p,entry]);
  fs.rmSync(to,{recursive:true,force:true});
  if(entry[0]==='absent')continue;
  fs.mkdirSync(path.dirname(to),{recursive:true});
  if(entry[0]==='link')fs.symlinkSync(entry[1],to);else{fs.copyFileSync(from,to);fs.chmodSync(to,entry[1]);}
 }
 // Deletions can leave old generated empty directories in a Git clone; mirror
 // their actual candidate absence as well as file bytes, without touching Git.
 function pruneAbsentEmpty(rel='') {
  for(const name of fs.readdirSync(path.join(clone,rel))) {
   if(!rel && name==='.git')continue;
   const child=path.join(rel,name),full=path.join(clone,child);
   if(!fs.lstatSync(full).isDirectory())continue;
   pruneAbsentEmpty(child);
   if(fs.readdirSync(full).length===0 && !fs.existsSync(path.join(source,child)))fs.rmdirSync(full);
  }
 }
 pruneAbsentEmpty();
 for(const [p,entry] of entries)assert.deepEqual(value(clone,p),entry,`snapshot changed ${p}`);
 const digest=crypto.createHash('sha256').update(JSON.stringify(entries)).digest('hex');
 run('bash',['build-plugin.sh','--check'],clone);
 run('bash',['install.sh',installed,'--platform','claude'],clone);
 for(const [name,canonical] of [['vibe','skills/vibe'],['flow','flow'],['spec','spec']])assert.equal(fs.readFileSync(`${installed}/.agents/skills/${name}/SKILL.md`,'utf8'),fs.readFileSync(`${clone}/${canonical}/SKILL.md`,'utf8'));
 const orders=run(process.execPath,[`${installed}/.agents/skills/flow/engine/cli.mjs`,'orders','feature.impl'],installed,{...process.env,CLAUDE_PROJECT_DIR:installed});assert.ok(orders.startsWith('skill=flow'));
 console.log(`PASS: disposable working-source snapshot sha256:${digest}; base ${base}; package and installed resource bytes verified; no commit/native marketplace proof`);
} finally {if(process.env.VIBE_KEEP_SNAPSHOT)console.log(`snapshot: ${work}`);else fs.rmSync(work,{recursive:true,force:true});}
