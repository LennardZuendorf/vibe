# PRD: Finalize AgentKit - CLI and Website

## Project Overview

**Goal**: Complete and publish AgentKit - a CLI tool and website for distributing AI agent development kits.

**Scope**: Merge legacy CLI into main project, finalize website, and prepare for publication.

## PHASE 1 DETAILED REQUIREMENTS: Monorepo Migration

### 1.1 Legacy Structure Analysis
**Current Legacy Structure:**
```
legacy/
├── package.json           # Root monorepo config
├── pnpm-workspace.yaml    # Workspace: packages/*, apps/*
├── turbo.json            # Turbo build configuration
├── packages/
│   └── registry/         # CLI package (@agent-kits/registry)
└── apps/                 # Empty directory
```

**Legacy Registry Package:**
- **Name**: `@agent-kits/registry`
- **Binary**: `agent-kits` command
- **Dependencies**: commander, ora, prompts
- **Build**: TypeScript → dist/index.js
- **Tests**: Vitest with coverage
- **Agent Kits**: `kits/base/engineer/` with full framework

### 1.2 Target Integration Requirements

**R1.1**: Merge Turbo configurations
- Combine legacy turbo.json tasks with current setup
- Ensure CLI build tasks are included
- Maintain task dependencies and caching

**R1.2**: Workspace integration  
- Both systems use `packages/*` and `apps/*` patterns
- Migrate CLI to `packages/cli/` (rename from registry)
- Preserve existing workspace structure

**R1.3**: Package migration specifics
- Move `legacy/packages/registry/` → `packages/cli/`
- Update package name: `@agent-kits/registry` → `agentkit`
- Update binary name: `agent-kits` → `agentkit`
- Preserve all dependencies and build configuration

**R1.4**: Agent kit preservation
- Move `legacy/packages/registry/kits/` → `packages/cli/kits/`
- Preserve engineer kit structure completely
- Maintain all .mdc rule files and templates
- Keep directory structure for kit installation

## Requirements (Updated)

### 1. Project Integration & Structure
- **R1.1**: Merge turbo.json configurations from both projects
- **R1.2**: Extend workspace to include migrated CLI package
- **R1.3**: Preserve existing agent kit framework (engineer kit)
- **R1.4**: Standardize branding to "AgentKit" (singular) throughout
- **R1.5**: Maintain CLI functionality with new package structure

### 2. CLI Requirements
- **R2.1**: CLI command structure: `npx agentkit add [kit-name]`
- **R2.2**: Maintain existing functionality from legacy/packages/registry
- **R2.3**: Prepare CLI for npm publication while keeping in monorepo
- **R2.4**: Update package.json to use "agentkit" as package name
- **R2.5**: Ensure CLI installs to `.cursor/` directory as before
- **R2.6**: Preserve TypeScript build process and testing setup

### 3. Website Requirements
- **R3.1**: Keep website minimal with focused features:
  - Landing page (current hero + features)
  - Legal pages (Privacy Policy, Terms of Service)
  - Link to GitHub repository
  - Basic agent kits overview
- **R3.2**: Update branding to "AgentKit" throughout
- **R3.3**: Ensure CLI command in hero matches actual command
- **R3.4**: Add documentation page for CLI usage

### 4. Agent Kits Content
- **R4.1**: Keep existing "engineer" kit unchanged
- **R4.2**: Preserve complete framework structure:
  - main-agent.md
  - rules/ directory with all .mdc files
  - templates/ directory
  - Complete orchestration system
- **R4.3**: Ensure agent kit installation works with new structure

### 5. Publication Readiness
- **R5.1**: CLI package ready for npm publication
- **R5.2**: Website deployable and functional
- **R5.3**: Documentation complete for basic usage
- **R5.4**: Legal pages present for public usage

## Migration Success Criteria

### Monorepo Integration
1. **Turbo Build**: All packages build successfully with turbo
2. **Workspace Function**: pnpm workspace commands work across all packages
3. **CLI Package**: New agentkit package builds and functions identically
4. **Test Suite**: All tests pass in new structure
5. **Dependencies**: No broken dependencies or import paths

### CLI Functionality Preservation
1. **Command Compatibility**: `agentkit add engineer` works identically to original
2. **File Installation**: Agent kit files install to correct `.cursor/` location
3. **Git Integration**: Git repository detection works correctly
4. **User Experience**: All prompts, spinners, and output formatting preserved

### Agent Kit Integrity
1. **Framework Preservation**: Complete multi-mode framework (Plan/Architect/Code/Test)
2. **File Structure**: All .mdc rule files and templates intact
3. **Installation Process**: Kit installation creates correct directory structure
4. **Memory System**: .memory/ integration works as designed

## Technical Constraints (Updated)
- Must preserve exact agent kit functionality
- CLI binary name changes from `agent-kits` to `agentkit`
- Package name changes from `@agent-kits/registry` to `agentkit`
- Must maintain TypeScript build process
- Must preserve test coverage and configuration
- Monorepo structure must support both website and CLI development

## Out of Scope
- Modifying agent kit framework logic
- Changing CLI core functionality
- Advanced website features beyond basic requirements
- Performance optimizations beyond current state 