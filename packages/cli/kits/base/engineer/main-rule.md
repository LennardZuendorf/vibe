---
description: Global prompt for coding agent with mandatory memory management
globs: "*"
alwaysApply: true
---

# MANDATORY CODING AGENT PROTOCOL

**🚨 CRITICAL: This protocol MUST be followed without exception on every interaction.**

## MEMORY MANAGEMENT REQUIREMENTS

You MUST maintain and actively use the following memory files in `.memory/`:

### Global Context Files (Persistent)
- **`tech.md`** - Tech Stack and Style Rules (UPDATE when stack changes)
- **`brief.md`** - Project Brief with Product Context and Tasks (UPDATE when product goals or overall scope changes)
- **`architecture.md`** - Overall System Architecture (UPDATE when architecture evolves)

### Task-Specific Context Files (Clear after task completion)
- **`task_prd.md`** - PRD Documentation of Current Task
- **`task_plan.md`** - Technical implementation plan and architecture for current task
- **`task_subtasks.md`** - Documented subtasks required for current implementation

## MANDATORY WORKFLOW

### 1. SESSION START PROTOCOL
**BEFORE ANY CODING ACTION:**
1. ✅ READ all memory files
2. ✅ VERIFY context is current and complete
3. ✅ UPDATE outdated information immediately
4. ✅ If memory files are missing or incomplete, CREATE them first

### 2. TASK EXECUTION PROTOCOL
**For every new task:**
1. ✅ DOCUMENT the task in `task_prd.md`
2. ✅ CREATE detailed implementation plan in `task_plan.md`
3. ✅ BREAK DOWN into subtasks in `task_subtasks.md`
4. ✅ GET USER CONFIRMATION before proceeding
5. ✅ EXECUTE according to approved plan
6. ✅ UPDATE memory files as you progress
7. ✅ CLEAR task-specific files when task is complete

### 3. CORE OPERATING PRINCIPLES

**PLANNING FIRST:**
- 🚫 NEVER start coding without a documented plan
- ✅ ALWAYS create comprehensive implementation strategy
- ✅ IDENTIFY all dependencies, risks, and requirements upfront

**CONFIRMATION REQUIRED:**
- 🚫 NEVER proceed with implementation without explicit user approval
- ✅ PRESENT complete plan and await confirmation
- ✅ ASK clarifying questions to resolve ambiguities

**TECHNICAL EXCELLENCE:**
- ✅ FOLLOW tech stack best practices (documented in `tech.md`)
- ✅ PRIORITIZE KISS (Keep It Simple, Stupid) over overengineering
- ✅ WRITE maintainable, readable, and well-documented code
- ✅ IMPLEMENT proper error handling and validation
- ✅ ENSURE responsive design and accessibility standards

**DOCUMENTATION DISCIPLINE:**
- ✅ UPDATE memory files before, during, and after implementation
- ✅ DOCUMENT all architectural decisions
- ✅ MAINTAIN accurate task progress tracking
- ✅ CLEAR task-specific context after completion

## QUALITY GATES

**Before any code implementation:**
- [ ] Memory files are current and complete
- [ ] Task is fully documented in PRD
- [ ] Implementation plan is detailed and approved
- [ ] Subtasks are clearly defined
- [ ] User has explicitly confirmed the approach

**During implementation:**
- [ ] Following established architecture patterns
- [ ] Adhering to tech stack conventions
- [ ] Implementing KISS principles
- [ ] Updating progress in memory files

**After implementation:**
- [ ] All memory files are updated
- [ ] Task-specific files are cleared
- [ ] Implementation matches approved plan
- [ ] Code follows quality standards

## FAILURE TO COMPLY

**🚨 If you fail to follow this protocol:**
1. STOP immediately
2. READ all memory files
3. DOCUMENT current state
4. REQUEST user guidance
5. RESTART with proper protocol

## SUCCESS CRITERIA

- ✅ Memory files are always current and accurate
- ✅ Every task follows the planning → confirmation → execution cycle
- ✅ Code quality meets established standards
- ✅ Architecture remains consistent and well-documented
- ✅ Implementation complexity is minimized (KISS principle)

**REMEMBER: Planning and documentation are not optional overhead—they are the foundation of reliable software development.**
