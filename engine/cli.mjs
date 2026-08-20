#!/usr/bin/env node
// engine/cli.mjs — vibe CLI entry point: arg parse, subcommand dispatch,
// error taxonomy, exit codes. The only process entry for the JS engine.
//
// Exit codes: 0 success (including --help); 1 a named error (unknown
// subcommand, a subcommand not implemented yet, or any other failure —
// dispatch() always resolves to a CliError or an unexpected error, never an
// unhandled rejection).

import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// The four commands this unit wires dispatch for. Later units add the
// module at engine/commands/<name>.mjs; until then dispatch reports a clear
// "not implemented yet" instead of a raw module-resolution error.
const COMMANDS = ['state', 'orders', 'doctrine', 'doctor'];

class CliError extends Error {
  constructor(message) {
    super(message);
    this.name = 'CliError';
  }
}

function printHelp() {
  const lines = [
    'vibe — spec-first workflow harness CLI',
    '',
    'Usage: vibe <command> [args...]',
    '',
    'Commands:',
    ...COMMANDS.map((name) => `  ${name}`),
    '',
    'Options:',
    '  -h, --help    show this help and exit',
  ];
  console.log(lines.join('\n'));
}

// A missing-module error names the specifier it failed to resolve in its
// message (Node gives no structured field for this). Only treat the error
// as "not implemented yet" when the missing specifier IS the command module
// itself — otherwise the command module exists but one of ITS OWN imports is
// broken, which is a real bug and must not be swallowed as "not implemented".
function missingSpecifierIsCommandModule(err, modulePath) {
  const match = /Cannot find module '([^']+)'/.exec(err && err.message ? err.message : '');
  if (!match) return false;
  const missing = match[1];
  const missingPath = missing.startsWith('file://') ? fileURLToPath(missing) : missing;
  return path.resolve(missingPath) === path.resolve(modulePath);
}

async function loadCommand(name) {
  const modulePath = path.join(__dirname, 'commands', `${name}.mjs`);
  try {
    return await import(pathToFileURL(modulePath).href);
  } catch (err) {
    const isMissingModule =
      err && (err.code === 'ERR_MODULE_NOT_FOUND' || err.code === 'MODULE_NOT_FOUND');
    if (isMissingModule && missingSpecifierIsCommandModule(err, modulePath)) {
      throw new CliError(`'${name}' is not implemented yet.`);
    }
    throw err;
  }
}

// Resolves argv to an exit code. Throws CliError for named failures; any
// other thrown error is treated as unexpected by the caller. Never leaves a
// rejected promise unhandled — every await here is inside this function's
// own try/catch at the call site in main().
async function dispatch(argv) {
  const [sub, ...rest] = argv;

  if (sub === undefined || sub === '--help' || sub === '-h') {
    printHelp();
    return 0;
  }

  if (!COMMANDS.includes(sub)) {
    throw new CliError(`unknown subcommand '${sub}'. Run 'vibe --help' for the command list.`);
  }

  const mod = await loadCommand(sub);
  const run = mod.default ?? mod.run;
  if (typeof run !== 'function') {
    throw new CliError(`'${sub}' is not implemented yet.`);
  }

  const code = await run(rest);
  return typeof code === 'number' ? code : 0;
}

async function main() {
  try {
    process.exitCode = await dispatch(process.argv.slice(2));
  } catch (err) {
    if (err instanceof CliError) {
      console.error(`vibe: ${err.message}`);
      process.exitCode = 1;
      return;
    }
    console.error('vibe: unexpected error:', err && err.stack ? err.stack : err);
    process.exitCode = 1;
  }
}

main().catch((err) => {
  // Defense in depth: main() already catches everything dispatch() can
  // throw, so this only fires if main() itself misbehaves.
  console.error('vibe: fatal error:', err && err.stack ? err.stack : err);
  process.exitCode = 1;
});
