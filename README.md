# AgentKit

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen?style=for-the-badge)](https://github.com/lzuendorf/agent-kit)
[![NPM Version](https://img.shields.io/npm/v/agentkit?style=for-the-badge)](https://www.npmjs.com/package/agentkit)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Deployed on Vercel](https://img.shields.io/badge/Website-agentkit.dev-black?style=for-the-badge&logo=vercel)](https://agentkit.dev)

AgentKit is a framework for building, testing, and sharing AI agents. It provides a CLI for managing "AgentKits"—reusable sets of prompts, rules, and configurations—and a Next.js-based website for documentation and community sharing.

## Key Features

- **Modular AgentKits**: Package your agent's logic into reusable Kits.
- **CLI Tool**: Easily add, manage, and share AgentKits in your projects.
- **Multi-Mode Framework**: Structure your agent's behavior with distinct modes like `plan`, `architect`, `code`, and `test`.
- **Turborepo Structure**: A clean, efficient monorepo for managing the website and CLI.
- **Extensible**: Designed to be customized and extended.

## Getting Started

### Installation

Install the AgentKit CLI globally to start using it in your projects.

```bash
npm install -g agentkit
```

### Add an AgentKit to Your Project

Navigate to your project's root directory and use the `add` command to install a pre-built AgentKit.

```bash
npx agentkit add engineer
```

This command will add the `engineer` AgentKit to a `.cursor` directory in your project, setting up the necessary prompts and configuration files.

## Monorepo Structure

This repository is a Turborepo containing the following packages:

- `app`: The Next.js application for the [agentkit.dev](https://agentkit.dev) website.
- `packages/cli`: The `agentkit` CLI tool.
- `packages/registry`: The registry for available AgentKits.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.

1.  Fork the repository.
2.  Create your feature branch (`git checkout -b feature/AmazingFeature`).
3.  Commit your changes (`git commit -m 'Add some AmazingFeature'`).
4.  Push to the branch (`git push origin feature/AmazingFeature`).
5.  Open a pull request.

## License

This project is licensed under the MIT License.
