# AgentKit Monorepo

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen?style=for-the-badge)](https://github.com/agentkit/agentkit)
[![NPM Version](https://img.shields.io/npm/v/@agentkit/cli?style=for-the-badge)](https://www.npmjs.com/package/@agentkit/cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Website](https://img.shields.io/badge/Website-agentkit.dev-black?style=for-the-badge&logo=vercel)](https://agentkit.dev)

AgentKit is a developer-first framework and CLI for building, syncing, and sharing AI agent rules, prompts, and configurations. This monorepo contains both the web application and CLI package.

## 🏗️ Monorepo Structure

This repository uses [Turborepo](https://turbo.build) for efficient monorepo management:

\`\`\`
├── apps/
│   └── web/                 # Next.js web application (agentkit.dev)
├── packages/
│   └── cli/                 # AgentKit CLI package (@agentkit/cli)
├── package.json             # Root package.json with workspace scripts
├── turbo.json              # Turborepo configuration
└── pnpm-workspace.yaml     # PNPM workspace configuration
\`\`\`

## 🚀 Quick Start

### Prerequisites

- Node.js 18+ 
- PNPM 9+

### Installation

\`\`\`bash
# Clone the repository
git clone https://github.com/agentkit/agentkit.git
cd agentkit

# Install dependencies
pnpm install

# Build all packages
pnpm build
\`\`\`

### Development

\`\`\`bash
# Start development servers for all apps
pnpm dev

# Start only the web app
pnpm app:dev

# Start only the CLI in watch mode
pnpm cli:dev
\`\`\`

## 📦 Packages

### Web Application (`apps/web`)

The main website at [agentkit.dev](https://agentkit.dev) built with:

- **Next.js 15** - React framework with App Router
- **Tailwind CSS** - Utility-first CSS framework
- **shadcn/ui** - Re-usable UI components
- **TypeScript** - Type safety

#### Development Commands

\`\`\`bash
# Start development server
pnpm app:dev

# Build for production
pnpm app:build

# Deploy to Vercel
pnpm app:deploy
\`\`\`

### CLI Package (`packages/cli`)

The `@agentkit/cli` package for managing AgentKits:

- **Commander.js** - CLI framework
- **TypeScript** - Type safety
- **Vitest** - Testing framework
- **Biome** - Linting and formatting

#### Development Commands

\`\`\`bash
# Build the CLI
pnpm cli:build

# Run in development mode
pnpm cli:dev

# Run tests
pnpm cli:test

# Publish to npm
pnpm cli:publish
\`\`\`

#### CLI Usage

\`\`\`bash
# Install globally
npm install -g @agentkit/cli

# Or use with npx
npx @agentkit/cli add engineer

# Initialize a new project
agentkit init

# Add an AgentKit
agentkit add engineer

# List available kits
agentkit list

# Remove a kit
agentkit remove engineer
\`\`\`

## 🛠️ Development Workflow

### Building

\`\`\`bash
# Build all packages
pnpm build

# Build specific package
pnpm cli:build
pnpm app:build
\`\`\`

### Testing

\`\`\`bash
# Run all tests
pnpm test

# Run CLI tests
pnpm cli:test
\`\`\`

### Linting & Formatting

\`\`\`bash
# Lint all packages
pnpm lint

# Fix linting issues
pnpm lint:fix

# Format code
pnpm format
\`\`\`

### Cleaning

\`\`\`bash
# Clean all build artifacts
pnpm clean
\`\`\`

## 🚢 Deployment

### Web Application

The web application is automatically deployed to Vercel on push to main:

\`\`\`bash
# Manual deployment
pnpm app:deploy
\`\`\`

### CLI Package

The CLI is published to npm:

\`\`\`bash
# Build and publish
pnpm cli:publish
\`\`\`

**Publishing Checklist:**
1. Update version in `packages/cli/package.json`
2. Update CHANGELOG.md
3. Run `pnpm cli:build` to ensure it builds
4. Run `pnpm cli:test` to ensure tests pass
5. Run `pnpm cli:publish`

## 📁 AgentKit Structure

AgentKits are structured packages containing:

\`\`\`
engineer/
├── README.md              # Kit documentation
├── main-agent.md         # Main agent configuration
├── main-rule.md          # Primary rules
├── mcp.json             # MCP configuration
├── rules/               # Additional rules
│   ├── modes/          # Mode-specific rules
│   └── orchestration.mdc
└── templates/          # Template files
    ├── plan_template.md
    └── prd_template.md
\`\`\`

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes
4. Run tests: `pnpm test`
5. Commit changes: `git commit -m 'Add amazing feature'`
6. Push to branch: `git push origin feature/amazing-feature`
7. Open a Pull Request

### Development Guidelines

- Use TypeScript for type safety
- Follow existing code style (Biome configuration)
- Write tests for new features
- Update documentation as needed
- Use conventional commit messages

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Links

- **Website**: [agentkit.dev](https://agentkit.dev)
- **Documentation**: [docs.agentkit.dev](https://docs.agentkit.dev)
- **NPM Package**: [@agentkit/cli](https://www.npmjs.com/package/@agentkit/cli)
- **GitHub**: [github.com/agentkit/agentkit](https://github.com/agentkit/agentkit)

---

Built with ❤️ by the AgentKit team
\`\`\`



































































```typescriptreact file="next.config.mjs" isDeleted="true"
...deleted...
