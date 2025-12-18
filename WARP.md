# PM Canvas - Project Rules

## Project Identity

**Name**: PM Canvas
**Purpose**: AI-optimized workspace for Product Managers collaborating with AI coding agents
**Primary Users**: Product Managers, Product Owners, Technical PMs, Solo Founders

## Core Philosophy

PM Canvas is a **documentation hub and multi-platform template** that helps Product Managers leverage AI agents (Cursor, Claude, Warp) effectively for product development tasks. It generalizes the proven patterns from fl-ai-toolbox (a working Cursor implementation) and provides cross-platform support, templates, and documentation for the broader PM community.

### Key Principles

1. **PM-First**: All templates, workflows, and structures are optimized for PM work, not engineering
2. **AI-Native**: Designed from the ground up for AI collaboration, based on proven fl-ai-toolbox patterns
3. **Context-Rich**: Extensive use of memory files to maintain product knowledge
4. **Multi-Platform**: Support for Cursor, Warp, Claude, and other AI assistants
5. **Documentation-First**: Clear documentation of architectural patterns and best practices
6. **Community-Driven**: Framework for sharing templates and workflows

## Repository Structure

```
pm-canvas/
├── starters/             # Platform-specific starter templates
│   ├── cursor/           # Cursor-specific (based on fl-ai-toolbox)
│   ├── warp/             # Warp-specific rules
│   └── claude/           # Claude-specific patterns
├── templates/            # Generalized PM document templates
├── workflows/            # PM workflow guides (platform-agnostic)
├── patterns/             # Architectural patterns from fl-ai-toolbox
├── examples/             # Real-world examples
├── app/                  # Documentation website (Next.js)
├── components/           # React components for website
└── docs/                 # Comprehensive documentation
```

## Relationship with fl-ai-toolbox

### fl-ai-toolbox: The Reference Implementation
**Location**: `/Users/lennard.zuendorf/dev/fl-ai-toolbox/`
**Status**: Working Cursor workspace, Phase 1 complete
**Focus**: Technical PM for commercial aviation/flight booking

fl-ai-toolbox has proven implementations of:
- Intelligent orchestration (core.mdc, index.mdc)
- Action/Mode system
- Memory management
- MCP integrations (Jira, Confluence, MySQL)
- Workspace structure
- Knowledge base

### PM Canvas: The Documentation & Template Hub
PM Canvas takes fl-ai-toolbox's proven patterns and:
1. **Documents** the architectural patterns
2. **Generalizes** for multiple AI platforms (not just Cursor)
3. **Abstracts** industry-specific knowledge to generic templates
4. **Simplifies** for easier adoption
5. **Shares** with the community

## Working with This Repository

### For Development (Meta-Level)
When working on PM Canvas itself, you're:
- **Extracting patterns** from fl-ai-toolbox
- **Creating documentation** for those patterns
- **Building templates** that work across platforms
- **Developing a website** to showcase the approach
- **Establishing a community** framework

### For PM Users (End-User)
When PMs use PM Canvas, they will:
1. **Learn** from the documentation
2. **Choose** a platform-specific starter (cursor/warp/claude)
3. **Customize** with their product context
4. **Use** templates and workflows
5. **Contribute** back to the community

## Development Guidelines

### Template Development
- **Location**: `/templates/<category>/<template-name>.md`
- **Format**: Markdown with clear sections and placeholders
- **Variables**: Use `{{variable_name}}` for AI to fill
- **Examples**: Include example outputs in comments
- **PM-Focused**: Write for PM audience, not engineers

### Workflow Development
- **Location**: `/workflows/<workflow-type>/<workflow-name>.md`
- **Structure**: Step-by-step with clear objectives
- **Context**: Reference relevant memory files
- **Templates**: Link to applicable templates
- **Validation**: Include quality gates

### Memory File Templates
- **Purpose**: Provide structure for PMs to document context
- **Categories**: Product, Feature, Stakeholder contexts
- **Format**: Markdown with clear sections
- **Guidance**: Include instructions for what to document

### Agent Rules (`.cursor/`)
- **Focus**: PM use cases, not code generation
- **Language**: Clear instructions for AI behavior
- **Context**: Leverage memory files effectively
- **Quality**: Ensure outputs meet PM standards

## Technology Decisions

### Website (Next.js)
- **Purpose**: Documentation, examples, community showcase
- **Audience**: PMs considering PM Canvas
- **Content**: Getting started, templates, workflows, examples
- **Style**: Clean, professional, PM-friendly (not developer-heavy)

### CLI (Future/Optional)
- **Purpose**: Workspace management utilities
- **Commands**: `init`, `template`, `sync`, `review`
- **Priority**: Lower priority, repository template is primary
- **Use Case**: Power users who want automation

### No Backend Required
- PM Canvas is a static repository template
- No services, databases, or APIs needed
- Pure client-side/local usage
- Optional integrations via CLI in future

## Content Standards

### Writing Style
- **Clarity**: Simple, direct language
- **PM-Centric**: Use PM terminology (features, requirements, users)
- **Action-Oriented**: Focus on what to do, not theory
- **Examples**: Always include concrete examples

### Template Quality
- **Complete**: Cover all essential sections
- **Flexible**: Adaptable to different products/contexts
- **Guided**: Clear instructions for each section
- **Professional**: Ready to share with stakeholders

### Workflow Quality
- **Practical**: Based on real PM workflows
- **Step-by-Step**: Clear progression
- **Decision Points**: Explicit choices and trade-offs
- **Outcomes**: Clear deliverables

## AI Agent Behavior

When working with this repository, AI agents should:

1. **Understand Context**: This is a PM workspace template, not an app to build
2. **PM Focus**: Generate PM content (PRDs, specs, docs), not code implementations
3. **Use Memory**: Always check `.memory/` for product context before generating content
4. **Follow Templates**: Use existing template structures
5. **Quality Standards**: Match professional PM document quality
6. **Terminology**: Use PM language (features, users, stories) over eng language (functions, classes, methods)

## Common Tasks

### Creating a New Template
1. Identify PM use case and document type
2. Research standard format for that document type
3. Create template in appropriate `/templates/` subdirectory
4. Include clear section headers and placeholder guidance
5. Add example usage to documentation
6. Update website to showcase new template

### Creating a New Workflow
1. Identify common PM activity/process
2. Break down into clear steps
3. Reference applicable templates and memory files
4. Include decision points and alternatives
5. Define success criteria and outputs
6. Document in `/workflows/` directory

### Updating Agent Rules
1. Identify PM use case or behavior need
2. Write clear, specific instructions
3. Reference memory files for context
4. Test with real PM scenarios
5. Update `.cursor/` rules
6. Document in usage guide

## Migration Notes

### From AgentKit (Old Direction)
This repository is transitioning from:
- **Was**: CLI tool to install agent configurations for developers
- **Now**: Complete workspace template for Product Managers

### What to Keep
- Memory management structure (adapt for PM context)
- Multi-mode framework concept (adapt for PM workflows)
- Template/rule organization patterns
- Next.js website infrastructure

### What to Transform
- Engineer-focused content → PM-focused content
- Code generation rules → Document generation rules
- Installation CLI → Workspace management CLI (optional)
- Developer templates → PM templates

### What to Remove/Archive
- `packages/cli/kits/base/engineer/` - developer-focused kit
- Engineer-specific agent rules
- Code-centric workflows

## Success Criteria

A successful PM Canvas provides:
1. **Immediate Value**: PM can clone and use on day one
2. **Clear Structure**: Obvious where things go and how to use them
3. **Quality Templates**: Professional-grade document templates
4. **Effective Workflows**: Proven PM processes
5. **AI Optimization**: Works seamlessly with AI agents
6. **Documentation**: Clear usage instructions
7. **Community**: Growing collection of contributed templates

## Questions to Ask

When adding features or content, ask:
1. **Is this PM-focused?** (vs. engineer-focused)
2. **Does it use memory effectively?** (context preservation)
3. **Is it template-driven?** (reusable patterns)
4. **Will PMs understand it?** (clarity for audience)
5. **Does it work with AI?** (AI collaboration optimized)

## Important Distinctions

**PM Canvas is NOT:**
- A project management tool (like Jira/Linear)
- A documentation platform (like Confluence/Notion)
- A developer tool (like Cursor alone)
- A code generator

**PM Canvas IS:**
- A structured workspace for PM-AI collaboration
- A template library for PM deliverables
- A context management system for products
- A workflow guide for PM activities
- A repository that works with AI coding agents

## Repository Naming

While the repository is currently named "agent-kit", we are renaming to "pm-canvas". When making changes:
- Use "PM Canvas" in user-facing content
- Update references from "AgentKit" to "PM Canvas"
- Maintain consistent branding
- Plan for repository rename on GitHub
