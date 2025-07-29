#!/usr/bin/env node
import { execSync } from 'child_process';
import { Command } from 'commander';
import fs from 'fs';
import ora from 'ora';
import path, { dirname } from 'path';
import prompts from 'prompts';
import { fileURLToPath } from 'url';
const __dirname = dirname(fileURLToPath(import.meta.url));
const program = new Command();
async function copyRecursiveAsync(src, dest, force) {
    const exists = fs.existsSync(src);
    const stats = exists && fs.statSync(src);
    const isDirectory = exists && stats && stats.isDirectory();
    if (isDirectory) {
        if (!fs.existsSync(dest)) {
            fs.mkdirSync(dest, { recursive: true });
        }
        const entries = fs.readdirSync(src);
        for (const entry of entries) {
            await copyRecursiveAsync(path.join(src, entry), path.join(dest, entry), force);
        }
    }
    else {
        if (fs.existsSync(dest) && !force) {
            const response = await prompts({
                type: 'confirm',
                name: 'overwrite',
                message: `File '${dest}' already exists. Overwrite?`,
                initial: false,
            });
            if (response && response.overwrite) {
                fs.copyFileSync(src, dest);
                console.log(`  Overwritten: ${dest}`);
            }
            else {
                console.log(`  Skipped: ${dest}`);
            }
        }
        else {
            fs.copyFileSync(src, dest);
            console.log(`  Copied: ${dest}`);
        }
    }
}
function isGitRepository() {
    try {
        execSync('git rev-parse --is-inside-work-tree', { stdio: 'ignore' });
        return true;
    }
    catch (e) {
        return false;
    }
}
function getGitRoot() {
    try {
        return execSync('git rev-parse --show-toplevel').toString().trim();
    }
    catch (e) {
        console.error('\n❌ Error: Could not determine Git repository root.\n', e);
        process.exit(1);
    }
}
function generateDirectoryMap(dir, indent = '') {
    let map = '';
    const files = fs.readdirSync(dir) || [];
    files.sort(); // Sort for consistent output
    for (let i = 0; i < files.length; i++) {
        const file = files[i];
        const filePath = path.join(dir, file);
        const isLast = i === files.length - 1;
        const prefix = indent + (isLast ? '└── ' : '├── ');
        map += prefix + file + '\n';
        const stat = fs.statSync(filePath);
        if (stat && typeof stat.isDirectory === 'function' && stat.isDirectory()) {
            map += generateDirectoryMap(filePath, indent + (isLast ? '    ' : '│   '));
        }
    }
    return map;
}
program
    .name('agentkit-registry')
    .description('A CLI tool to install AgentKits into a repository.')
    .version('1.0.0');
program
    .command('add')
    .description('Add an AgentKit to your repository.')
    .argument('[kit-name]', 'The name of the AgentKit to install (e.g., engineer)')
    .option('-f, --force', 'Overwrite existing files without prompting.')
    .option('-t, --target <dir>', 'Installation directory relative to the repo root', '.cursor')
    .action(async (kitName, options) => {
    if (!kitName) {
        console.error('No AgentKit name provided.');
        process.exit(1);
    }
    const spinner = ora('Checking for Git repository...').start();
    if (!isGitRepository()) {
        spinner.fail('Not inside a Git repository.');
        console.error("Please navigate to your repository's root directory and try again.");
        process.exit(1);
    }
    spinner.succeed('Git repository found.');
    const gitRoot = getGitRoot();
    const targetPath = path.join(gitRoot, '.cursor');
    const sourcePath = path.join(__dirname, '..', 'kits', 'base', kitName);
    const availableKits = ['engineer'];
    if (!availableKits.includes(kitName)) {
        spinner.fail(`AgentKit ${kitName} not found.`);
        process.exit(1);
    }
    let overwriteAll = !!options.force;
    if (fs.existsSync(targetPath)) {
        spinner.stop();
        console.log('A .cursor directory already exists at your repository root.');
        if (!options.force) {
            const { overwrite } = await prompts({
                type: 'confirm',
                name: 'overwrite',
                message: 'Overwrite existing .cursor directory?',
                initial: false,
            });
            if (overwrite) {
                fs.rmSync(targetPath, { recursive: true, force: true });
                spinner.succeed('Existing .cursor directory deleted.');
                overwriteAll = true;
            }
            else {
                console.log('Proceeding with individual file overwrite prompts.');
            }
        }
        else {
            fs.rmSync(targetPath, { recursive: true, force: true });
            spinner.succeed('Existing .cursor directory deleted.');
        }
    }
    else {
        spinner.succeed('No existing .cursor directory found.');
    }
    const { proceed } = await prompts({
        type: 'confirm',
        name: 'proceed',
        message: 'Install AgentKit into your repository?',
        initial: true,
    });
    if (!proceed) {
        console.log('Installation cancelled by user.');
        process.exit(0);
    }
    if (!fs.existsSync(targetPath)) {
        fs.mkdirSync(targetPath, { recursive: true });
    }
    await copyRecursiveAsync(sourcePath, targetPath, overwriteAll);
    spinner.succeed('AgentKit installed successfully!');
    console.log('--- Installed Files ---');
    console.log(generateDirectoryMap(targetPath));
});
const isDirectRun = path.resolve(process.argv[1] || '') === fileURLToPath(import.meta.url);
if (isDirectRun) {
    program.parse(process.argv);
}
export { program };
