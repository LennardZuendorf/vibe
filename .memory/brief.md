# Project Brief: PM Canvas - AI Workspace for Product Managers

## Project Overview

**PM Canvas** is a specialized workspace and repository template designed for Product Managers to effectively collaborate with AI agents (Cursor, Claude, Warp) on product development tasks. It provides structured workflows, templates, and potentially a CLI for managing PM-specific activities.

## Vision

Transform product management work by providing PMs with:
- A ready-to-use repository structure optimized for AI collaboration
- PM-specific workflows for requirements, planning, and documentation
- Intelligent context management through memory files
- Templates and frameworks for common PM deliverables
- Optional CLI for workspace management and scaffolding

## Target Audience

**Primary**: Product Managers working with AI coding agents
**Secondary**: Product-focused roles (Product Owners, Technical PMs, Startup Founders)

**User Personas**:
1. **Technical PM**: Needs to communicate requirements clearly to AI agents
2. **Non-technical PM**: Wants to leverage AI for technical tasks without deep coding knowledge
3. **Solo Founder**: Building products with AI assistance, needs structure

## Core Value Proposition

### For Product Managers
- **Structure**: Pre-built workspace for AI-assisted product development
- **Templates**: Ready-made formats for PRDs, specs, user stories, roadmaps
- **AI Optimization**: Context management that helps AI understand PM intent
- **Workflows**: Proven patterns for collaborating with AI agents

### For Organizations
- **Consistency**: Standardized approach to AI-assisted product work
- **Knowledge**: Captured in structured memory files
- **Efficiency**: Faster onboarding, clearer communication with dev teams

## Key Features

### 1. Repository Structure
- **Memory Management**: Structured `.memory/` for context persistence
- **Templates Library**: Pre-built templates for PM deliverables
- **Workflows**: Guided processes for common PM tasks
- **Agent Rules**: Optimized AI agent configurations for PM work

### 2. PM-Specific Templates
- Product Requirements Documents (PRDs)
- User Stories & Acceptance Criteria
- Feature Specifications
- Technical Documentation
- Roadmap Planning
- Stakeholder Communications

### 3. AI Collaboration Framework
- **Context Preservation**: Memory files that maintain project knowledge
- **Intent Translation**: Structures that help AI understand PM needs
- **Iterative Refinement**: Workflows supporting AI-assisted iteration
- **Quality Gates**: Validation steps for AI-generated content

### 4. Optional CLI (Future)
- `pm-canvas init` - Initialize new PM workspace
- `pm-canvas template <type>` - Generate template documents
- `pm-canvas sync` - Sync with project management tools
- `pm-canvas review` - Quality check PM documents

## Differentiation

**vs. Traditional PM Tools**: Deep AI integration, not just task tracking
**vs. Developer AI Tools**: PM-focused, not engineering-focused
**vs. Documentation Tools**: Active workspace, not passive repository

## Success Metrics

- **Adoption**: Clones/forks of the repository template
- **Engagement**: Active usage of templates and workflows
- **Community**: Contributions of new templates and patterns
- **Quality**: PM satisfaction with AI-generated content

## Current State

**Repository Status**: Clarifying relationship with fl-ai-toolbox implementation

### Existing Implementation: fl-ai-toolbox
The **fl-ai-toolbox** repository (`/Users/lennard.zuendorf/dev/fl-ai-toolbox/`) has already implemented most of the PM workspace vision:

**What fl-ai-toolbox Has Built:**
- ✅ Intelligent orchestration system (core.mdc, index.mdc)
- ✅ Action system for discrete tasks (ticket-writing, repo-search)
- ✅ Mode system for persistent behaviors (data-analyst, code-inspector, product-owner)
- ✅ Memory management (.memory/ with prd.md, design_doc.md, brief.md)
- ✅ Workspace structure (workspaces/projects/, analysis/, code-research/)
- ✅ Knowledge base system (knowledge/ with glossaries)
- ✅ MCP integrations (Jira, Confluence, MySQL)
- ✅ Repository management (submodules for code access)
- ✅ Taskfile automation for setup/maintenance

**Technology Context:**
- Platform: Cursor IDE (not Warp/Claude)
- Structure: Opinionated workspace with .cursor/rules/
- Focus: Technical PM for commercial aviation/flight booking product
- Status: Phase 1 (Foundation) complete, working implementation

### PM Canvas Repositioning
Given that fl-ai-toolbox has implemented the core vision, PM Canvas should evolve to:

**Option A: Multi-Platform Template**
- Generalized version supporting Cursor, Warp, and Claude
- Abstract the orchestration patterns from fl-ai-toolbox
- Provide platform-specific implementations

**Option B: Documentation & Community Hub**
- Website showcasing the PM workspace approach
- Documentation for the fl-ai-toolbox architecture
- Template library and community contributions
- Getting started guides for different AI tools

**Option C: Starter Template**
- Simplified, batteries-included version of fl-ai-toolbox
- Quick-start template for new PM AI workspaces
- Less opinionated, more configurable
- Industry-agnostic templates and workflows

### Components to Leverage from fl-ai-toolbox
- Orchestration architecture (core.mdc pattern)
- Action/Mode distinction
- Memory system patterns
- Workspace folder structure
- Knowledge base approach
- MCP integration patterns

### What PM Canvas Adds (Phase 1 Focus)
- Cursor-first implementation with proper rules and commands
- Generalized PM templates (not aviation-specific)
- Shell script automation for workspace setup
- Clear documentation of patterns
- Simplified structure for easy adoption

## Current Phase: Foundation (Phase 1)

**Goal:** Create functional Cursor workspace with essential PM tools

**Approach:**
- Shell scripts for setup automation
- Cursor rules based on fl-ai-toolbox patterns
- Essential PM templates (PRD, user stories, roadmaps)
- Memory system for context management
- Workspaces structure for organizing work

**Timeline:** 1-2 days

**Future phases (Taskfile, TypeScript CLI, multi-platform) deferred until Phase 1 validated**
