import fs from "fs-extra"
import path from "path"
import chalk from "chalk"
import ora from "ora"
import prompts from "prompts"

interface InitOptions {
  directory?: string
}

export async function initCommand(options: InitOptions) {
  const spinner = ora("Initializing AgentKit project...").start()

  try {
    const targetDir = options.directory || ".cursor"
    const kitPath = path.resolve(targetDir)

    // Check if directory already exists
    if (await fs.pathExists(kitPath)) {
      spinner.stop()
      const response = await prompts({
        type: "confirm",
        name: "overwrite",
        message: `Directory ${targetDir} already exists. Overwrite?`,
        initial: false,
      })

      if (!response.overwrite) {
        console.log(chalk.yellow("Initialization cancelled"))
        return
      }

      spinner.start("Initializing AgentKit project...")
    }

    // Create basic structure
    await fs.ensureDir(kitPath)
    await fs.ensureDir(path.join(kitPath, "rules"))
    await fs.ensureDir(path.join(kitPath, "templates"))

    // Create basic files
    const readmeContent = `# AgentKit Project

This directory contains your AgentKit configuration.

## Structure

- \`rules/\` - Agent rules and modes
- \`templates/\` - Template files
- \`main-agent.md\` - Main agent configuration
- \`main-rule.md\` - Primary rules

## Usage

Use \`agentkit add <kit-name>\` to add pre-built kits to this project.
`

    await fs.writeFile(path.join(kitPath, "README.md"), readmeContent)

    const mainAgentContent = `# Main Agent Configuration

This is your main agent configuration file.
`

    await fs.writeFile(path.join(kitPath, "main-agent.md"), mainAgentContent)

    spinner.succeed("AgentKit project initialized successfully")
    console.log(chalk.green(`\nProject created at: ${kitPath}`))
    console.log(chalk.gray("You can now add AgentKits using: agentkit add <kit-name>"))
  } catch (error) {
    spinner.fail("Failed to initialize AgentKit project")
    console.error(chalk.red(error instanceof Error ? error.message : "Unknown error"))
  }
}
