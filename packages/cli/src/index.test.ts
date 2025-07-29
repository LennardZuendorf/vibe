import { execSync } from 'child_process';
import type { Command } from 'commander';
import fs from 'fs';
import ora from 'ora';
import prompts from 'prompts';
import { vi } from 'vitest';

// Mock external modules
vi.mock('child_process');
vi.mock('fs');
vi.mock('prompts');
vi.mock('ora');

const mockExecSync = execSync as vi.Mock;
const mockExistsSync = fs.existsSync as vi.Mock;
const mockMkdirSync = fs.mkdirSync as vi.Mock;
const mockCopyFileSync = fs.copyFileSync as vi.Mock;
const mockRmSync = fs.rmSync as vi.Mock;
const mockPrompts = prompts as vi.MockedFunction<typeof prompts>;
const mockOra = ora as vi.Mock;

// Mock ora spinner methods
const mockSpinner = {
  start: vi.fn().mockReturnThis(),
  succeed: vi.fn().mockReturnThis(),
  fail: vi.fn().mockReturnThis(),
  stop: vi.fn().mockReturnThis(),
};
mockOra.mockReturnValue(mockSpinner);

// Capture console output
let consoleLogSpy: vi.SpyInstance;
let consoleErrorSpy: vi.SpyInstance;

describe('AgentKit CLI', () => {
  let program: Command;
  let exitSpy: vi.SpyInstance;

  beforeEach(async () => {
    vi.clearAllMocks();
    consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    // Mock process.exit
    exitSpy = vi.spyOn(process, 'exit').mockImplementation(((code?: number) => {
      throw new Error(`process.exit: ${code}`);
    }) as any);

    // Clear and re-import the module for each test to reset Commander's state
    vi.resetModules();
    const cliModule = await import('./index');
    program = cliModule.program;
  });

  afterEach(() => {
    consoleLogSpy.mockRestore();
    consoleErrorSpy.mockRestore();
    exitSpy.mockRestore();
  });

  it('should exit with error if no AgentKit name is provided', async () => {
    process.argv = ['node', 'index.js', 'add'];

    await expect(program.parseAsync(process.argv)).rejects.toThrow('process.exit: 1');
    expect(consoleErrorSpy).toHaveBeenCalledWith('No AgentKit name provided.');
  });

  it('should exit with error if not in a git repository', async () => {
    mockExecSync.mockImplementation(() => {
      throw new Error('Not a git repo');
    });
    process.argv = ['node', 'index.js', 'add', 'engineer'];

    await expect(program.parseAsync(process.argv)).rejects.toThrow('process.exit: 1');

    expect(mockOra).toHaveBeenCalledWith('Checking for Git repository...');
    expect(mockSpinner.start).toHaveBeenCalled();
    expect(mockSpinner.fail).toHaveBeenCalledWith('Not inside a Git repository.');
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      expect.stringContaining("Please navigate to your repository's root directory and try again.")
    );
  });

  it('should exit with error if AgentKit name is not available', async () => {
    mockExecSync.mockReturnValueOnce('true');
    mockExecSync.mockReturnValueOnce('/mock/git/root');

    process.argv = ['node', 'index.js', 'add', 'unknown-kit'];

    await expect(program.parseAsync(process.argv)).rejects.toThrow('process.exit: 1');
    expect(mockSpinner.fail).toHaveBeenCalledWith('AgentKit unknown-kit not found.');
  });

  it('should install AgentKit successfully when target directory exists', async () => {
    mockExecSync.mockReturnValueOnce('true'); // isGitRepository
    mockExecSync.mockReturnValueOnce('/mock/git/root'); // getGitRoot
    mockExistsSync.mockReturnValue(true); // target exists

    // Mock prompts - first for overwrite, then for proceed
    mockPrompts.mockResolvedValueOnce({ overwrite: true }).mockResolvedValueOnce({ proceed: true });

    process.argv = ['node', 'index.js', 'add', 'engineer'];

    // Should complete successfully without calling process.exit
    await program.parseAsync(process.argv);

    expect(mockSpinner.stop).toHaveBeenCalled();
    expect(mockSpinner.succeed).toHaveBeenCalledWith('AgentKit installed successfully!');
    expect(exitSpy).not.toHaveBeenCalled();
  });
});
