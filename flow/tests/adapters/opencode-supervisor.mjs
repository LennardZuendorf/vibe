// vibe-spec: literal-file
// This live group leader anchors ownership after the native CLI exits.
import { spawn } from 'node:child_process';
import { writeSync } from 'node:fs';

const [command, ...args] = JSON.parse(process.argv[2]);
process.on('SIGTERM', () => {});
let reported = false;
function report(result) {
  if (reported) return;
  reported = true;
  try { writeSync(3, `${JSON.stringify(result)}\n`); }
  catch { cleanup(); }
}
let cleaning = false;
function cleanup() {
  if (cleaning) return;
  cleaning = true;
  // The anchor is still alive, so its PGID cannot have been reused.
  process.kill(-process.pid, 'SIGTERM');
  setTimeout(() => process.kill(-process.pid, 'SIGKILL'), 100);
}
process.stdin.once('end', cleanup);
process.stdin.once('error', cleanup);
process.stdin.resume();
const child = spawn(command, args, { stdio: ['pipe', 'pipe', 'pipe'] });
child.stdin.on('error', () => {});
process.stdin.on('data', data => { if (child.stdin.writable) child.stdin.write(data); });
process.stdin.once('end', () => child.stdin.end());
child.stdout.pipe(process.stdout);
child.stderr.pipe(process.stderr);
child.once('error', error => report({ error: error.message }));
child.once('exit', (code, signal) => report({ code, signal }));
child.once('close', () => {
  process.stdout.end();
  process.stderr.end();
});
