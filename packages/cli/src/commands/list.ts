import fs from "fs-extra"
import path from "path"
import chalk from "chalk"
import { fileURLToPath } from "url"
import { dirname } from "path"

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

interface ListOptions {
  remote?: boolean
}

export async function listCommand(options: ListOptions) {
  try {
    if (options.remote) {
      console.log(chalk.blue("Available AgentKits from registry:"))
      // TODO: Implement remote registry listing
      console.log(chalk.yellow("Remote registry listing not yet implemented"))
      return
    }

    // List local kits
    const kitsDir = path.join(__dirname, "../../kits/base")

    if (!(await fs.pathExists(kitsDir))) {
      console.log(chalk.yellow("No kits directory found"))
      return
    }

    const kits = await fs.readdir(kitsDir)
    const validKits = []

    for (const kit of kits) {
      const kitPath = path.join(kitsDir, kit)
      const stat = await fs.stat(kitPath)
      if (stat.isDirectory() && !kit.startsWith(".")) {
        validKits.push(kit)
      }
    }

    if (validKits.length === 0) {
      console.log(chalk.yellow("No AgentKits found"))
      return
    }

    console.log(chalk.blue("Available AgentKits:"))
    for (const kit of validKits) {
      const kitPath = path.join(kitsDir, kit)
      const readmePath = path.join(kitPath, "README.md")

      let description = "No description available"
      if (await fs.pathExists(readmePath)) {
        const readme = await fs.readFile(readmePath, "utf-8")
        const match = readme.match(/^# .+\n\n(.+)/)
        if (match) {
          description = match[1].split("\n")[0]
        }
      }

      console.log(chalk.cyan(`  ${kit}`))
      console.log(chalk.gray(`    ${description}`))
    }
  } catch (error) {
    console.error(chalk.red("Failed to list AgentKits"))
    console.error(chalk.red(error instanceof Error ? error.message : "Unknown error"))
  }
}
