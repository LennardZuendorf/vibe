"use client"

import { useState } from "react"
import { Copy, Check, Terminal } from "lucide-react"
import { Button } from "@/components/ui/button"

const steps = [
  {
    title: "Install AgentKit CLI",
    command: "npm install -g agentkit",
    description: "Install the AgentKit CLI globally to manage agent configurations across platforms",
  },
  {
    title: "Initialize Registry",
    command: "agentkit init --registry https://github.com/your-org/agent-kits",
    description: "Connect to your team's registry or use the public registry for shared kits",
  },
  {
    title: "Add Agent Kits",
    command: "agentkit add cursor-engineer claude-writer",
    description: "Install platform-specific kits and sync them with your development environment",
  },
  {
    title: "Sync Across Platforms",
    command: "agentkit sync --platforms cursor,claude",
    description: "Keep your agent configurations synchronized across all your tools",
  },
]

export function InstallationSection() {
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null)

  const copyCommand = async (command: string, index: number) => {
    try {
      await navigator.clipboard.writeText(command)
      setCopiedIndex(index)
      setTimeout(() => setCopiedIndex(null), 2000)
    } catch (err) {
      console.error("Failed to copy: ", err)
    }
  }

  return (
    <section className="w-full py-16 md:py-24 bg-zinc-950 border-b border-zinc-800">
      <div className="container px-4 md:px-6">
        <div className="text-center mb-12">
          <h2 className="text-3xl md:text-4xl font-bold text-white mb-4">Start syncing in minutes</h2>
          <p className="text-zinc-400 text-lg max-w-2xl mx-auto">
            Set up AgentKit to manage and sync your AI agent configurations across platforms and teams.
          </p>
        </div>

        <div className="max-w-4xl mx-auto">
          <div className="grid gap-6">
            {steps.map((step, index) => (
              <div key={index} className="flex gap-6">
                {/* Step Number */}
                <div className="flex-shrink-0 w-8 h-8 bg-white text-black rounded-full flex items-center justify-center font-bold text-sm">
                  {index + 1}
                </div>

                {/* Step Content */}
                <div className="flex-1">
                  <h3 className="text-xl font-semibold text-white mb-2">{step.title}</h3>
                  <p className="text-zinc-400 mb-4">{step.description}</p>

                  {/* Terminal */}
                  <div className="bg-zinc-900 border border-zinc-800 rounded-lg overflow-hidden">
                    <div className="flex items-center justify-between px-4 py-2 bg-zinc-800 border-b border-zinc-700">
                      <div className="flex items-center space-x-2">
                        <Terminal className="w-4 h-4 text-zinc-400" />
                        <span className="text-sm text-zinc-400">terminal</span>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => copyCommand(step.command, index)}
                        className="h-6 px-2 text-zinc-400 hover:text-white"
                      >
                        {copiedIndex === index ? <Check className="w-3 h-3" /> : <Copy className="w-3 h-3" />}
                      </Button>
                    </div>
                    <div className="p-4 font-mono text-sm">
                      <div className="flex items-center space-x-2">
                        <span className="text-green-400">$</span>
                        <span className="text-white">{step.command}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  )
}
