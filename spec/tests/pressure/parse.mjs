// Reads Claude stream-json on stdin; prints the first Skill name invoked
// before any Edit/Write/MultiEdit/NotebookEdit, or "none".
import { createInterface } from 'node:readline';
const WRITES = new Set(['Edit', 'Write', 'MultiEdit', 'NotebookEdit']);
let first = null;
for await (const line of createInterface({ input: process.stdin })) {
  if (first) continue;
  let ev; try { ev = JSON.parse(line); } catch { continue; }
  const content = ev?.message?.content;
  if (ev.type !== 'assistant' || !Array.isArray(content)) continue;
  for (const c of content) {
    if (c.type !== 'tool_use') continue;
    if (c.name === 'Skill') { const n = String(c.input?.skill ?? 'none'); first = /(^|:)spec$/.test(n) ? 'spec' : n; break; }
    if (WRITES.has(c.name)) { first = 'none'; break; }
  }
}
process.stdout.write(`${first ?? 'none'}\n`);
