import { execSync } from 'child_process';
import fs from 'fs';
import prompts from 'prompts';
import ora from 'ora';
import { vi } from 'vitest';
// Mock external modules
vi.mock('child_process');
vi.mock('fs');
vi.mock('prompts');
vi.mock('ora');
const mockExecSync = execSync;
const mockExistsSync = fs.existsSync;
const mockMkdirSync = fs.mkdirSync;
const mockCopyFileSync = fs.copyFileSync;
const mockRmSync = fs.rmSync;
const mockPrompts = prompts;
const mockOra = ora;
// Mock ora spinner methods
const mockSpinner = {
    start: vi.fn().mockReturnThis(),
    succeed: vi.fn().mockReturnThis(),
    fail: vi.fn().mockReturnThis(),
    stop: vi.fn().mockReturnThis(),
};
mockOra.mockReturnValue(mockSpinner);
// Capture console output
let consoleLogSpy;
let consoleErrorSpy;
describe('agent-kits CLI', () => {
    let program;
    let exitSpy;
    beforeEach(() => {
        vi.clearAllMocks();
        consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => { });
        consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => { });
        // Mock process.exit
        exitSpy = vi.spyOn(process, 'exit').mockImplementation(((code) => { throw new Error(`process.exit: ${code}`); }));
        // Clear and re-import the module for each test to reset Commander's state
        vi.resetModules();
        const cliModule = require('../src/index');
        program = cliModule.program;
    });
    afterEach(() => {
        consoleLogSpy.mockRestore();
        consoleErrorSpy.mockRestore();
        exitSpy.mockRestore();
    });
    it('should exit with error if no kit name is provided', async () => {
        process.argv = ['node', 'index.js', 'add'];
        await expect(program.parseAsync(process.argv)).rejects.toThrow('process.exit: 1');
        expect(consoleErrorSpy).toHaveBeenCalledWith(expect.stringContaining('No agent kit name provided'));
    });
    it('should exit with error if not in a git repository', async () => {
        mockExecSync.mockImplementation(() => { throw new Error('Not a git repo'); });
        process.argv = ['node', 'index.js', 'add', 'engineer'];
        await expect(program.parseAsync(process.argv)).rejects.toThrow('process.exit: 1');
        expect(mockSpinner.start).toHaveBeenCalledWith('Checking for Git repository...');
        expect(mockSpinner.fail).toHaveBeenCalledWith('Not inside a Git repository.');
        expect(consoleErrorSpy).toHaveBeenCalledWith(expect.stringContaining('Please navigate to your repository\'s root directory and try again.'));
    });
    it('should install kit successfully when .cursor does not exist', async () => {
        mockExecSync.mockReturnValueOnce('true'); // isGitRepository
        mockExecSync.mockReturnValueOnce('/mock/git/root'); // getGitRoot
        mockExistsSync.mockReturnValue(false); // .cursor does not exist, sourcePath exists
        mockPrompts.mockResolvedValueOnce({ proceed: true }); // Confirm installation
        process.argv = ['node', 'index.js', 'add', 'engineer'];
        await program.parseAsync(process.argv);
        expect(mockSpinner.succeed).toHaveBeenCalledWith('No existing .cursor directory found.');
        expect(mockMkdirSync).toHaveBeenCalled();
        expect(mockCopyFileSync).toHaveBeenCalled();
        expect(mockSpinner.succeed).toHaveBeenCalledWith(expect.stringContaining('agent kit installed successfully!'));
        expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('--- Installed Files ---'));
    });
    it('should overwrite .cursor completely if user confirms', async () => {
        mockExecSync.mockReturnValueOnce('true'); // isGitRepository
        mockExecSync.mockReturnValueOnce('/mock/git/root'); // getGitRoot
        mockExistsSync.mockReturnValueOnce(true); // .cursor exists
        mockExistsSync.mockReturnValue(true); // sourcePath exists (for copyRecursiveAsync)
        mockPrompts.mockResolvedValueOnce({ overwrite: true }); // Confirm overwrite .cursor
        mockPrompts.mockResolvedValueOnce({ proceed: true }); // Confirm installation
        process.argv = ['node', 'index.js', 'add', 'engineer'];
        await program.parseAsync(process.argv);
        expect(mockSpinner.stop).toHaveBeenCalled(); // Spinner stops before prompt
        expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('A .cursor directory already exists'));
        expect(mockPrompts).toHaveBeenCalledWith(expect.objectContaining({ name: 'overwrite' }));
        expect(mockRmSync).toHaveBeenCalledWith(expect.stringContaining('.cursor'), { recursive: true, force: true });
        expect(mockSpinner.succeed).toHaveBeenCalledWith('Existing .cursor directory deleted.');
        expect(mockCopyFileSync).toHaveBeenCalled();
    });
    it('should prompt for individual file overwrite if .cursor exists and user declines full overwrite', async () => {
        mockExecSync.mockReturnValueOnce('true'); // isGitRepository
        mockExecSync.mockReturnValueOnce('/mock/git/root'); // getGitRoot
        mockExistsSync.mockReturnValueOnce(true); // .cursor exists
        mockExistsSync.mockReturnValueOnce(true); // file in .cursor exists
        mockExistsSync.mockReturnValue(true); // sourcePath exists (for copyRecursiveAsync)
        mockPrompts.mockResolvedValueOnce({ overwrite: false }); // Decline overwrite .cursor
        mockPrompts.mockResolvedValueOnce({ proceed: true }); // Confirm installation
        mockPrompts.mockResolvedValueOnce({ overwrite: true }); // Confirm individual file overwrite
        process.argv = ['node', 'index.js', 'add', 'engineer'];
        await program.parseAsync(process.argv);
        expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('Proceeding with individual file overwrite prompts.'));
        expect(mockPrompts).toHaveBeenCalledWith(expect.objectContaining({ name: 'overwrite', message: expect.stringContaining('already exists. Overwrite?') }));
        expect(mockCopyFileSync).toHaveBeenCalled();
    });
    it('should overwrite all files without prompting if --force flag is used', async () => {
        mockExecSync.mockReturnValueOnce('true'); // isGitRepository
        mockExecSync.mockReturnValueOnce('/mock/git/root'); // getGitRoot
        mockExistsSync.mockReturnValueOnce(true); // .cursor exists
        mockExistsSync.mockReturnValue(true); // sourcePath exists (for copyRecursiveAsync)
        mockPrompts.mockResolvedValueOnce({ proceed: true }); // Confirm installation (only one prompt due to --force)
        process.argv = ['node', 'index.js', 'add', 'engineer', '--force'];
        await program.parseAsync(process.argv);
        expect(mockRmSync).toHaveBeenCalledWith(expect.stringContaining('.cursor'), { recursive: true, force: true });
        expect(mockSpinner.succeed).toHaveBeenCalledWith('Existing .cursor directory deleted.');
        expect(mockPrompts).not.toHaveBeenCalledWith(expect.objectContaining({ name: 'overwrite' })); // No overwrite prompt
        expect(mockCopyFileSync).toHaveBeenCalled();
    });
    it('should exit if user cancels installation at disclaimer prompt', async () => {
        mockExecSync.mockReturnValueOnce('true'); // isGitRepository
        mockExecSync.mockReturnValueOnce('/mock/git/root'); // getGitRoot
        mockExistsSync.mockReturnValue(false); // .cursor does not exist
        mockPrompts.mockResolvedValueOnce({ proceed: false }); // Cancel installation
        process.argv = ['node', 'index.js', 'add', 'engineer'];
        await expect(program.parseAsync(process.argv)).rejects.toThrow('process.exit: 0');
        expect(consoleLogSpy).toHaveBeenCalledWith('Installation cancelled by user.');
    });
    it('should exit with error if kit not found', async () => {
        mockExecSync.mockReturnValueOnce('true'); // isGitRepository
        mockExecSync.mockReturnValueOnce('/mock/git/root'); // getGitRoot
        mockExistsSync.mockReturnValueOnce(false); // .cursor does not exist
        mockExistsSync.mockReturnValueOnce(false); // sourcePath does not exist
        mockPrompts.mockResolvedValueOnce({ proceed: true }); // Confirm installation
        process.argv = ['node', 'index.js', 'add', 'nonexistent-kit'];
        await expect(program.parseAsync(process.argv)).rejects.toThrow('process.exit: 1');
        expect(mockSpinner.fail).toHaveBeenCalledWith(expect.stringContaining('Agent kit ' + 'nonexistent-kit' + ' not found.'));
    });
});
