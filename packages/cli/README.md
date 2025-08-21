# @agentkit/cli

The official CLI for AgentKit - a framework for building, syncing, and sharing AI agent rules, prompts, and configurations.

## Installation

Install globally via npm:

\`\`\`bash
npm install -g @agentkit/cli
\`\`\`

Or use directly with npx:

\`\`\`bash
npx @agentkit/cli add engineer
\`\`\`

## Usage

### Initialize a new AgentKit project

\`\`\`bash
agentkit init
\`\`\`

This creates a `.cursor` directory with the basic AgentKit structure.

### Add an AgentKit

\`\`\`bash
agentkit add engineer
\`\`\`

Installs the "engineer" AgentKit to your project's `.cursor` directory.

### List available AgentKits

\`\`\`bash
agentkit list
\`\`\`

Shows all locally available AgentKits.

### Remove an AgentKit

\`\`\`bash
agentkit remove engineer
\`\`\`

Removes the specified AgentKit from your project.

### Update AgentKits

\`\`\`bash
agentkit update
agentkit update engineer  # Update specific kit
\`\`\`

Updates AgentKits to their latest versions.

## Options

Most commands support these options:

- `-d, --directory <dir>` - Specify target directory (default: `.cursor`)
- `-s, --source <source>` - Use custom source for kits

## AgentKit Structure

An AgentKit typically contains:

- `main-agent.md` - Main agent configuration
- `main-rule.md` - Primary rules
- `rules/` - Additional rules and modes
- `templates/` - Template files
- `mcp.json` - MCP configuration (if applicable)

## Development

This CLI is part of the AgentKit monorepo. To contribute:

1. Clone the repository
2. Install dependencies: `pnpm install`
3. Build the CLI: `pnpm cli:build`
4. Test locally: `pnpm cli:dev`

## Publishing

To publish a new version:

\`\`\`bash
pnpm cli:publish
\`\`\`

This builds the CLI and publishes it to npm.
