// Tasks 3/5/6 integration: legacy hook paths never certify an adopted run.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { runGateHook, runGuardHook } from '../../engine/commands/hook.mjs';
const root=fs.mkdtempSync(path.join(os.tmpdir(),'vibe-adopted-hooks-'));
const runtime=path.join(root,'.vibe/run/flow');const dir=path.join(root,'.agents/skills/flow');
fs.mkdirSync(path.join(runtime,'evidence'),{recursive:true});fs.mkdirSync(dir,{recursive:true});
fs.writeFileSync(path.join(runtime,'evidence/feature-test.md'),'');
try {
 for(const kind of ['v2','versionless','corrupt','missing']) {
  fs.mkdirSync(path.join(runtime,'runs'),{recursive:true});
  const cursor=path.join(runtime,'state.json');fs.rmSync(cursor,{force:true});
  if(kind!=='missing')fs.writeFileSync(cursor,kind==='corrupt'?'{broken':JSON.stringify({...(kind==='v2'?{v:2}:{}),flow:'feature',phase:'verify',feature:'test'}));
  for(const active of [false,true]) {
   const payload=JSON.stringify({stop_hook_active:active});
   assert.equal(runGateHook(root,dir,payload).code,2,`${kind}: Node Stop active=${active}`);
   assert.equal(runGuardHook(root,JSON.stringify({tool_name:'Write',tool_input:{file_path:'src/x'}})).code,2,`${kind}: Node guard`);
   for(const script of ['stop-gate','pre-tool-use-guard']) {
    const out=spawnSync('bash',[`flow/hooks-fallback/${script}.sh`],{env:{...process.env,CLAUDE_PROJECT_DIR:root},input:payload,encoding:'utf8'});assert.equal(out.status,2,`${kind}: shell ${script} ${out.stderr}`);
   }
  }
 }
 // The no-runtime shim must not silently allow when every delegate is absent.
 const standalone=path.join(root,'hook.sh');fs.copyFileSync('flow/bin/hook.sh',standalone);
 const tools=path.join(root,'tools');fs.mkdirSync(tools);
 for(const tool of ['bash','sh','dirname','uname','tr','cat']) {const found=spawnSync('which',[tool],{encoding:'utf8'});fs.symlinkSync(found.stdout.trim(),path.join(tools,tool));}
 const out=spawnSync('/bin/sh',[standalone,'stop-gate'],{env:{...process.env,PATH:tools,CLAUDE_PROJECT_DIR:root,VIBE_FLOW_BIN:'/missing',VIBE_ENGINE:'/missing'},input:'{"stop_hook_active":true}',encoding:'utf8'});
 assert.equal(out.status,2,out.stderr);
 console.log('PASS: adopted v2/versionless/corrupt/missing cursors deny legacy guard/Stop/reentry/no-runtime shim');
} finally {fs.rmSync(root,{recursive:true,force:true});}
