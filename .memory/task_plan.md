# Technical Architecture Plan: AgentKit Monorepo Migration

## Current State Analysis

### Legacy Monorepo Structure
```
legacy/
├── package.json                    # Monorepo root with workspaces
├── pnpm-workspace.yaml            # packages/*, apps/*
├── turbo.json                     # Complete build pipeline
├── pnpm-lock.yaml                 # Dependency lock file
└── packages/
    └── registry/                  # CLI package
        ├── src/index.ts           # CLI entry point
        ├── kits/base/engineer/    # Agent kit framework
        ├── package.json           # @agent-kits/registry
        ├── tsconfig.json          # TypeScript config
        ├── vitest.config.ts       # Test configuration
        └── dist/                  # Build output
```

### Current Main Project Structure
```
agent-kit/
├── package.json                   # Next.js app only
├── pnpm-workspace.yaml           # packages/*, apps/* (empty)
├── turbo.json                    # Same turbo config as legacy
├── app/                          # Next.js application
├── components/                   # React components
├── lib/                         # Utilities
├── .memory/                     # Agent memory files
└── packages/                    # Empty directory
```

## Target Architecture Design

### Unified Monorepo Structure
```
agent-kit/
├── package.json                   # Combined monorepo root
├── pnpm-workspace.yaml           # Unified workspace config
├── turbo.json                    # Enhanced turbo configuration  
├── pnpm-lock.yaml                # Combined dependencies
├── app/                          # Next.js website
├── components/                   # Shared React components
├── lib/                         # Shared utilities
├── .memory/                      # Agent memory files
└── packages/
    └── cli/                      # Migrated CLI package
        ├── src/                  # TypeScript source
        │   ├── index.ts          # CLI entry point
        │   └── types.d.ts        # Type definitions
        ├── kits/                 # Agent kit definitions
        │   └── base/
        │       └── engineer/     # Complete framework
        ├── dist/                 # Build output
        ├── package.json          # agentkit package
        ├── tsconfig.json         # TypeScript config
        ├── vitest.config.ts      # Test configuration
        └── README.md             # CLI documentation
```

## Migration Architecture Components

### 1. Package Configuration Architecture

#### Root package.json Structure
```json
{
  "name": "agentkit-monorepo",
  "private": true,
  "workspaces": ["packages/*", "apps/*"],
  "scripts": {
    "build": "turbo run build",
    "dev": "turbo run dev",
    "test": "turbo run test:run",
    "lint": "turbo run lint",
    "clean": "turbo run clean"
  },
  "devDependencies": {
    "turbo": "^2.0.0",
    "typescript": "^5"
  }
}
```

#### CLI Package Configuration
```json
{
  "name": "agentkit",
  "version": "1.0.0",
  "type": "module",
  "bin": {
    "agentkit": "./dist/index.js"
  },
  "main": "dist/index.js",
  "files": ["dist", "kits"],
  "dependencies": {
    "commander": "^12.0.0",
    "ora": "^8.0.1", 
    "prompts": "^2.4.2"
  }
}
```

### 2. Build System Architecture

#### Turbo Configuration Strategy
- **Merge Strategy**: Combine tasks from both turbo.json files
- **Task Dependencies**: Ensure CLI builds before website if needed
- **Caching Strategy**: Optimize for both development and CI

#### Enhanced turbo.json Structure
```json
{
  "$schema": "https://turbo.build/schema.json",
  "ui": "tui",
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "inputs": [
        "src/**/*.ts", 
        "src/**/*.tsx",
        "app/**/*.tsx",
        "components/**/*.tsx",
        "package.json", 
        "tsconfig.json"
      ],
      "outputs": ["dist/**", ".next/**"]
    },
    "dev": {
      "cache": false,
      "persistent": true
    },
    "test:run": {
      "dependsOn": ["build"],
      "inputs": ["src/**/*.ts", "src/**/*.test.ts"]
    },
    "lint": {
      "inputs": ["src/**/*.ts", "src/**/*.tsx", "app/**/*.tsx"]
    }
  }
}
```

### 3. CLI Architecture Preservation

#### Source Code Migration Strategy
- **File Mapping**: Direct 1:1 migration of source files
- **Import Path Updates**: Update relative imports if needed
- **Binary Configuration**: Update shebang and executable permissions

#### Agent Kit Framework Architecture
```
packages/cli/kits/base/engineer/
├── main-agent.md              # Core agent prompt
├── main-rule.md               # Main rule file
├── mcp.json                   # MCP configuration
├── README.md                  # Documentation
├── rules/                     # Rule definitions
│   ├── commit-rule.mdc        # Commit guidelines
│   ├── orchestration.mdc      # Main orchestration
│   └── modes/                 # Operation modes
│       ├── architect-mode.mdc # Design phase
│       ├── code-mode.mdc      # Implementation phase
│       ├── plan-mode.mdc      # Planning phase
│       ├── test-mode.mdc      # Testing phase
│       └── coding-rules/      # Coding standards
└── templates/                 # Document templates
    ├── plan_template.md       # Planning template
    └── prd_template.md        # PRD template
```

### 4. Dependency Management Architecture

#### Workspace Dependencies Strategy
- **Shared Dependencies**: Hoist common dependencies to root
- **Package-specific**: Keep CLI dependencies in package
- **Version Alignment**: Ensure TypeScript/Biome versions match

#### Lock File Strategy
- **Single Lock**: Use root pnpm-lock.yaml
- **Dependency Resolution**: Ensure no conflicts between packages

### 5. Build and Distribution Architecture

#### Development Workflow
```bash
# Development commands
pnpm dev              # Start website dev server
pnpm --filter cli dev # Watch CLI TypeScript compilation
pnpm build           # Build all packages
pnpm test            # Run all test suites
```

#### CLI Distribution Strategy
- **Build Target**: ES modules for Node.js
- **Binary Setup**: Proper shebang and permissions
- **File Inclusion**: Include kits/ directory in package
- **npm Publication**: Ready for `npm publish`

### 6. Testing Architecture

#### Test Strategy Preservation
- **Vitest Configuration**: Maintain existing test setup
- **Coverage**: Preserve coverage configuration
- **Test Files**: Migrate all existing tests

#### Cross-Package Testing
- **Isolation**: Each package tests independently
- **Integration**: Add integration tests for CLI installation

## Implementation Phases

### Phase 1: Structure Setup
1. **Create packages/cli directory**
2. **Update workspace configuration** 
3. **Merge turbo.json configurations**

### Phase 2: Package Migration
1. **Copy source files with structure preservation**
2. **Update package.json with new naming**
3. **Fix any import path issues**

### Phase 3: Build Integration
1. **Test build process**
2. **Verify CLI functionality**
3. **Validate agent kit installation**

### Phase 4: Validation
1. **End-to-end testing**
2. **Package publication readiness**
3. **Clean up legacy files**

## Risk Mitigation Strategies

### Functionality Preservation
- **Backup Strategy**: Keep legacy/ until validation complete
- **Incremental Testing**: Test each migration step
- **Rollback Plan**: Clear revert process if issues arise

### Dependency Conflicts
- **Version Pinning**: Pin critical dependency versions
- **Resolution Strategy**: Use pnpm overrides if needed
- **Isolation**: Maintain package boundary integrity

### Build System Issues
- **Incremental Validation**: Test turbo tasks individually
- **Cache Management**: Clear caches during migration
- **Output Validation**: Verify build artifacts match expectations

## Success Metrics

### Technical Validation
1. **Build Success**: All packages build without errors
2. **Test Passage**: All tests pass in new structure
3. **Functionality**: CLI works identically to original
4. **Performance**: Build times remain reasonable

### Integration Validation  
1. **Workspace Commands**: pnpm workspace operations work
2. **Turbo Tasks**: All turbo tasks execute correctly
3. **Package Resolution**: Dependencies resolve properly
4. **CLI Installation**: `npx agentkit add engineer` works 