# Tech Stack & Coding Standards

## Technology Stack

### Frontend (Next.js App)
- **Framework**: Next.js 15.2.4 with App Router
- **React**: React 19 with React DOM 19
- **TypeScript**: TypeScript 5+
- **Styling**: Tailwind CSS 3.4.17 with tailwindcss-animate
- **UI Components**: Radix UI primitives with custom component layer
- **Icons**: Lucide React
- **Fonts**: Geist font family
- **Theme**: next-themes for dark/light mode support

### CLI & Backend (Optional/Future)
- **Runtime**: Node.js with ES Modules
- **CLI Framework**: Commander.js
- **Package Manager**: pnpm with workspaces
- **Build System**: TypeScript compiler + Turbo for monorepo
- **Testing**: Vitest
- **Purpose**: Workspace management utilities for PMs

### Development Tools
- **Linting**: Biome (replacing ESLint/Prettier)
- **Monorepo**: Turbo + pnpm workspaces
- **Git Hooks**: Husky + lint-staged
- **Coverage**: Vitest coverage

## Project Structure Standards

### File Organization
```
/
├── app/                    # Next.js app directory
├── components/             # React components
│   ├── ui/                # Base UI components (shadcn/ui style)
│   └── [feature]/         # Feature-specific components
├── lib/                   # Utilities and helpers
├── packages/              # Monorepo packages
│   └── registry/          # CLI package
├── .memory/               # Agent memory files
└── legacy/                # To be merged/cleaned up
```

### Component Architecture
- **Base Components**: Radix UI + Tailwind in `/components/ui/`
- **Feature Components**: Composed components in `/components/`
- **Naming**: PascalCase for components, kebab-case for files
- **Exports**: Named exports preferred, default for pages

## Coding Conventions

### TypeScript Standards
- **Strict Mode**: Enabled
- **Type Safety**: Prefer explicit types over `any`
- **Interfaces**: Use for object shapes
- **Enums**: Use const assertions or union types

### React Patterns
- **Hooks**: Custom hooks in `/hooks/`
- **State**: useState for local, useContext for shared
- **Effects**: Minimize useEffect usage, prefer derived state
- **Props**: Interface definitions for all component props

### Styling Guidelines
- **Tailwind**: Utility-first approach
- **Components**: Use `cn()` utility for conditional classes
- **Responsive**: Mobile-first breakpoints
- **Dark Mode**: Support via next-themes

### CLI Standards (Future)
- **Commands**: PM-focused operations (init, template, sync, review)
- **Output**: Use ora spinners for long operations
- **Prompts**: Confirm destructive operations
- **Error Handling**: Graceful failures with helpful messages
- **Focus**: Workspace scaffolding and template generation

## Quality Gates

### Code Quality
- **Biome**: Automated linting and formatting
- **TypeScript**: No build errors
- **Tests**: Vitest for unit tests
- **Coverage**: Target 80%+ coverage for critical paths

### Performance
- **Bundle Size**: Monitor with Next.js analyzer
- **Core Web Vitals**: Maintain good scores
- **Loading**: Implement proper loading states

## Build & Deployment

### Development
```bash
pnpm dev          # Start Next.js dev server
pnpm build        # Build all packages
pnpm test         # Run test suite
pnpm lint         # Lint and format
```

### Production
- **Platform**: Vercel (auto-deploy from main branch)
- **Environment**: Node.js 18+
- **Repository Template**: GitHub template repository
- **CLI Distribution**: npm registry (optional future feature)

## Dependencies Management

### Keep Current
- Next.js (follow stable releases)
- React (stable versions)
- TypeScript (latest stable)
- Tailwind CSS (stable releases)

### Version Pinning
- UI components (Radix UI) - pin minor versions
- Build tools (Turbo, Biome) - pin patch versions
- CLI dependencies - pin for stability

## KISS Principle Application

### Simplicity First
- **Architecture**: Avoid over-engineering
- **Dependencies**: Minimize external packages
- **Components**: Single responsibility principle
- **Abstractions**: Only when clear benefit exists

### Pragmatic Choices
- **File Structure**: Flat when possible, nested when necessary
- **State Management**: Built-in hooks over external libraries
- **Styling**: Tailwind utilities over custom CSS
- **Testing**: Focus on critical user paths 