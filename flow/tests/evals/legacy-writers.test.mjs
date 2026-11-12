// Tasks 3/5 integration: legacy writers cannot overwrite upgraded runtime.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { runSet } from '../../engine/commands/state.mjs';
const root=fs.mkdtempSync(path.join(os.tmpdir(),'vibe-legacy-writer-'));
const dir=path.join(root,'.agents/skills/flow');const runtime=path.join(root,'.vibe/run/flow');
fs.mkdirSync(dir,{recursive:true});fs.mkdirSync(runtime,{recursive:true});
fs.copyFileSync('flow/state-machine.json',path.join(dir,'state-machine.json'));
const state=path.join(runtime,'state.json');
try {
 const v2=JSON.stringify({v:2,flow:'feature',phase:'impl',feature:'test',revision:1});fs.writeFileSync(state,v2);
 let node=runSet(dir,['idle']);assert.notEqual(node.code,0,'Node refuses v2');assert.equal(fs.readFileSync(state,'utf8'),v2);
 let shell=spawnSync('bash',['flow/scripts/set-state.sh','idle'],{env:{...process.env,CLAUDE_PROJECT_DIR:root,VIBE_FLOW_BIN:'/missing'},encoding:'utf8'});assert.notEqual(shell.status,0,'shell refuses v2');assert.equal(fs.readFileSync(state,'utf8'),v2);
 const v1=JSON.stringify({flow:'idle',phase:'idle',feature:null});fs.writeFileSync(state,v1);fs.writeFileSync(path.join(runtime,'runtime.lock'),'someone else');
 node=runSet(dir,['feature.design','test']);assert.notEqual(node.code,0,'Node respects runtime lock');assert.equal(fs.readFileSync(state,'utf8'),v1);assert.equal(fs.readFileSync(path.join(runtime,'runtime.lock'),'utf8'),'someone else');
 shell=spawnSync('bash',['flow/scripts/set-state.sh','feature.design','test'],{env:{...process.env,CLAUDE_PROJECT_DIR:root,VIBE_FLOW_BIN:'/missing'},encoding:'utf8'});assert.notEqual(shell.status,0,'shell respects runtime lock');assert.equal(fs.readFileSync(state,'utf8'),v1);assert.equal(fs.readFileSync(path.join(runtime,'runtime.lock'),'utf8'),'someone else');
 fs.unlinkSync(path.join(runtime,'runtime.lock'));fs.mkdirSync(path.join(runtime,'runs'));fs.writeFileSync(state,'{broken');
 node=runSet(dir,['idle']);assert.notEqual(node.code,0,'adoption marker protects corrupt cursor');assert.equal(fs.readFileSync(state,'utf8'),'{broken');
 shell=spawnSync('bash',['flow/scripts/set-state.sh','idle'],{env:{...process.env,CLAUDE_PROJECT_DIR:root},encoding:'utf8'});assert.notEqual(shell.status,0);assert.equal(fs.readFileSync(state,'utf8'),'{broken');
 console.log('PASS: Node/shell refuse v2 and respect shared exclusive runtime lock');
} finally {fs.rmSync(root,{recursive:true,force:true});}
