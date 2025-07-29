# Engineer AgentKit

This is the `engineer` AgentKit, a pre-configured set of rules and prompts designed to help AI agents perform software engineering tasks. This kit is ideal for agents focused on code generation, refactoring, debugging, and technical documentation.

## What's Inside?

This AgentKit includes:

- **`main-agent.md`**: The core prompt that defines the Engineer agent's primary role and high-level instructions.
- **`rules/`**: A directory containing specific rules and guidelines for different engineering modes:
  - **`orchestration.mdc`**: Defines how the agent switches between different engineering modes.
  - **`modes/`**:
    - **`plan-mode.mdc`**: Guides the agent in creating a plan to approach a given task.
    - **`architect-mode.mdc`**: Helps the agent design the architecture for a software solution.
    - **`code-mode.mdc`**: Directs the agent in writing and implementing code.
    - **`test-mode.mdc`**: Instructs the agent on how to create and run tests for the code.
- **`templates/`**: Contains templates for common engineering documents or code structures.

## How to Use This AgentKit

Once installed, the files in this AgentKit are placed in your project's `.cursor/` directory (or your specified target directory). Your AI agent can then read and interpret these files to guide its behavior.

### Example Usage (Conceptual)

Your AI agent, when configured to use this AgentKit, might follow a workflow similar to this:

1.  **Understand the Request**: The agent reads `main-agent.md` to understand its role as an engineer.
2.  **Plan the Task**: It consults `rules/modes/plan-mode.mdc` to formulate a step-by-step plan.
3.  **Design the Solution**: It uses `rules/modes/architect-mode.mdc` to design the software components.
4.  **Implement the Code**: It refers to `rules/modes/code-mode.mdc` while writing the code.
5.  **Test the Implementation**: It applies `rules/modes/test-mode.mdc` to create and run tests.

### Customization

You can customize this AgentKit by editing the `.md` and `.mdc` files directly within your project's `.cursor/` directory. This allows you to tailor the agent's behavior to your specific needs and project conventions.

For example, you might:
- Modify `main-agent.md` to give your engineer agent a more specific persona.
- Add new rules to the `rules/` directory for handling specific coding standards or technologies.
- Create new templates in the `templates/` directory for frequently used code snippets or documentation formats.

## Feedback and Contributions

We welcome your feedback and contributions to improve this Engineer AgentKit. If you have suggestions for new rules, modes, or improvements, please consider contributing to the main AgentKit repository.