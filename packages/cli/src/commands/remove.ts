import fs from "fs-extra"
import path from "path"
import chalk from "chalk"
import ora from "ora"

interface RemoveOptions {
  directory?: string
}

export async function removeCommand(kitName: string, options: RemoveOptions) {
  const spinner = ora(`Removing AgentKit: ${kitName}`).start()

  try {
    const targetDir = options.directory || ".cursor"
    const kitPath = path.resolve(targetDir)

    if (!(await fs.pathExists(kitPath))) {
      spinner.fail(`No AgentKit installation found in ${targetDir}`)
      return
    }

    // Remove the kit directory
    await fs.remove(kitPath)

    spinner.succeed(`Successfully removed AgentKit: ${kitName}`)
    console.log(chalk.green(`Kit removed from: ${kitPath}`))
  } catch (error) {
    spinner.fail(`Failed to remove AgentKit: ${kitName}`)
    console.error(chalk.red(error instanceof Error ? error.message : "Unknown error"))
  }
}
