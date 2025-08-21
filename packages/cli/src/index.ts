#!/usr/bin/env node

import { Command } from "commander"
import { addCommand } from "./commands/add.js"
import { listCommand } from "./commands/list.js"
import { removeCommand } from "./commands/remove.js"
import { updateCommand } from "./commands/update.js"
import { initCommand } from "./commands/init.js"

const program = new Command()

program.name("agentkit").description("CLI for managing AgentKits - reusable AI agent configurations").version("1.0.0")

program
  .command("add")
  .description("Add an AgentKit to your project")
  .argument("<kit>", "Name of the kit to add")
  .option("-s, --source <source>", "Custom source for the kit")
  .option("-d, --directory <directory>", "Target directory (default: .cursor)")
  .action(addCommand)

program
  .command("list")
  .description("List available AgentKits")
  .option("-r, --remote", "List remote kits from registry")
  .action(listCommand)

program
  .command("remove")
  .description("Remove an AgentKit from your project")
  .argument("<kit>", "Name of the kit to remove")
  .option("-d, --directory <directory>", "Target directory (default: .cursor)")
  .action(removeCommand)

program
  .command("update")
  .description("Update an AgentKit to the latest version")
  .argument("[kit]", "Name of the kit to update (updates all if not specified)")
  .option("-d, --directory <directory>", "Target directory (default: .cursor)")
  .action(updateCommand)

program
  .command("init")
  .description("Initialize a new AgentKit project")
  .option("-d, --directory <directory>", "Target directory (default: .cursor)")
  .action(initCommand)

program.parse()
