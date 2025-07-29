# Project Brief: AgentKit

## Project Overview

This project is a **dual-purpose agent development platform** that combines:

1. **A Next.js website** - Marketing and documentation site for AgentKits
2. **A CLI registry system** - Tool for distributing and installing AI agent configurations

The project aims to provide developers with reusable components, rules, and modes for building AI agents, particularly focused on software development automation.

## Current State Analysis

### Main Project (Current Directory)
- **Type**: Next.js 15 application with React 19
- **Purpose**: Marketing/documentation website for AgentKits
- **Tech Stack**: Next.js, TypeScript, Tailwind CSS, Radix UI components
- **Status**: Active v0.dev project with auto-deployment to Vercel
- **URL**: https://agentkit.dev

### Legacy Project (/legacy)
- **Type**: Monorepo with CLI tool and AgentKit registry
- **Purpose**: CLI for installing agent configurations into repositories
- **Tech Stack**: TypeScript, Commander.js, Node.js CLI
- **Status**: Functional but needs integration with main project
- **Key Components**:
  - `agentkit-registry` - CLI tool for kit installation
  - AgentKits (currently: "engineer" kit)
  - Multi-mode development framework (Plan → Architect → Code → Test)

## Product Context

**Target Audience**: Developers building AI agents, particularly for software development automation

**Core Value Proposition**: 
- Reusable agent components and behaviors
- Structured multi-mode development lifecycle
- Easy installation via CLI
- Copy-paste customizable agent rules

**Key Features**:
1. **CLI Installation**: `npx agentkit add engineer`
2. **Agent Modes**: Plan, Architect, Code, Test modes with orchestration
3. **Memory Management**: Structured context files for agents
4. **Component Library**: Reusable agent behaviors and rules

## Integration Requirements

The two projects need to be merged to create a unified platform where:
- Website showcases and documents the AgentKits
- CLI provides the installation mechanism
- AgentKits are properly distributed and versioned
- Documentation is comprehensive and up-to-date

## Next Steps Priority

1. **Merge Projects**: Integrate legacy monorepo into main project structure
2. **Update CLI**: Ensure CLI works with new structure
3. **Documentation**: Create comprehensive docs for AgentKits
4. **Component Showcase**: Display available AgentKits on website
5. **Distribution**: Set up proper npm publishing workflow 