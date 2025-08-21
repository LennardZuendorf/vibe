import fs from "fs-extra"
import path from "path"
import chalk from "chalk"
import ora from "ora"
import { fileURLToPath } from "url"
import { dirname } from "path"

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

interface AddOptions {
  source?: string
  directory?: string
}

export async function addCommand(kitName: string, options: AddOptions) {
  const spinner = ora(`Adding AgentKit: ${kitName}`).start()

  try {
    const targetDir = options.directory || ".cursor"
    const kitPath = path.resolve(targetDir)

    // Ensure target directory exists
    await fs.ensureDir(kitPath)

    // Get kit source path
    const kitSourcePath = path.join(__dirname, "../../kits/base", kitName)

    if (!(await fs.pathExists(kitSourcePath))) {
      spinner.fail(`Kit "${kitName}" not found`)
      console.log(chalk.yellow("\nAvailable kits:"))
      const kitsDir = path.join(__dirname, "../../kits/base")
      if (await fs.pathExists(kitsDir)) {
        const availableKits = await fs.readdir(kitsDir)
        availableKits.forEach((kit) => {
          console.log(chalk.cyan(`  - ${kit}`))
        })
      }
      return
    }

    // Copy kit files
    await fs.copy(kitSourcePath, kitPath, {
      overwrite: true,
      filter: (src) => !src.includes(".DS_Store"),
    })

    spinner.succeed(`Successfully added AgentKit: ${kitName}`)
    console.log(chalk.green(`\nKit installed to: ${kitPath}`))
    console.log(chalk.gray("You can now use this AgentKit in your AI development workflow."))
  } catch (error) {
    spinner.fail(`Failed to add AgentKit: ${kitName}`)
    console.error(chalk.red(error instanceof Error ? error.message : "Unknown error"))
  }
}
