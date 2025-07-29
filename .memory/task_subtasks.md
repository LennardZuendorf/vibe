# Task Subtasks: AgentKit Monorepo Migration

## PHASE 1: STRUCTURE SETUP (IN PROGRESS)

### Subtask 1.1: Create CLI Package Directory ✅
- [x] Create `packages/cli/` directory structure

### Subtask 1.2: Update Workspace Configuration
- [ ] Update root `package.json` to include CLI workspace
- [ ] Verify `pnpm-workspace.yaml` includes packages/*
- [ ] Update root package.json scripts for monorepo

### Subtask 1.3: Merge Turbo Configurations
- [ ] Combine tasks from legacy and current turbo.json
- [ ] Add CLI-specific build tasks
- [ ] Ensure proper task dependencies

### Subtask 1.4: Migrate CLI Package
- [ ] Copy source files from legacy/packages/registry/src/
- [ ] Copy agent kits from legacy/packages/registry/kits/
- [ ] Create new packages/cli/package.json with agentkit branding
- [ ] Copy TypeScript and test configurations
- [ ] Update import paths and dependencies

### Subtask 1.5: Update CLI Branding
- [ ] Change package name to "agentkit"
- [ ] Update binary name to "agentkit"
- [ ] Update CLI descriptions and help text
- [ ] Update console output to use "AgentKit"

## VALIDATION CHECKLIST
- [ ] `packages/cli` directory created
- [ ] Workspace recognizes new package
- [ ] Turbo can build CLI package
- [ ] CLI installs and runs correctly
- [ ] Agent kit installation works 