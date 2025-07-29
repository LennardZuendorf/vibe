"use client"

import { useState } from "react"
import { Copy, Check } from "lucide-react"
import { Button } from "@/components/ui/button"

const agentKits = [
  {
    name: "Cursor Engineer",
    description: "Code review and debugging rules optimized for Cursor IDE",
    command: "npx agentkit add cursor-engineer",
    platform: "Cursor",
    preview: `// Cursor Engineer Kit
export const cursorEngineerKit = {
  name: "cursor-engineer",
  platform: "cursor",
  rules: [
    "Focus on code quality and best practices",
    "Provide actionable debugging steps",
    "Suggest performance optimizations"
  ],
  prompts: {
    codeReview: "Review this code for...",
    debugging: "Help debug this issue..."
  }
}`,
  },
  {
    name: "Claude Writer",
    description: "Content creation and editing prompts for Claude",
    command: "npx agentkit add claude-writer",
    platform: "Claude",
    preview: `// Claude Writer Kit
export const claudeWriterKit = {
  name: "claude-writer", 
  platform: "claude",
  rules: [
    "Maintain consistent tone and style",
    "Optimize for clarity and readability",
    "Provide constructive editing feedback"
  ],
  prompts: {
    contentGen: "Create content that...",
    editing: "Edit this text to improve..."
  }
}`,
  },
  {
    name: "N8N Workflow",
    description: "Automation agent for n8n workflow management",
    command: "npx agentkit add n8n-workflow",
    platform: "n8n",
    preview: `// N8N Workflow Kit
export const n8nWorkflowKit = {
  name: "n8n-workflow",
  platform: "n8n", 
  rules: [
    "Optimize workflow efficiency",
    "Handle error cases gracefully",
    "Provide clear automation logic"
  ],
  triggers: ["webhook", "schedule", "manual"],
  actions: ["process", "transform", "notify"]
}`,
  },
]

export function AgentShowcase() {
  const [activeAgent, setActiveAgent] = useState(0)
  const [copied, setCopied] = useState(false)

  const copyCommand = async (command: string) => {
    try {
      await navigator.clipboard.writeText(command)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (err) {
      console.error("Failed to copy: ", err)
    }
  }

  return (
    <section className="w-full py-16 md:py-24 bg-black border-b border-zinc-800">
      <div className="container px-4 md:px-6">
        <div className="text-center mb-12">
          <h2 className="text-3xl md:text-4xl font-bold text-white mb-4">Platform-Specific AgentKits</h2>
          <p className="text-zinc-400 text-lg max-w-2xl mx-auto">
            Pre-configured agent kits optimized for specific platforms and use cases. Install, sync, and customize for
            your workflow.
          </p>
        </div>

        <div className="grid lg:grid-cols-2 gap-8 max-w-6xl mx-auto">
          {/* Agent Selector */}
          <div className="space-y-4">
            {agentKits.map((kit, index) => (
              <div
                key={kit.name}
                className={`p-6 rounded-lg border cursor-pointer transition-all ${
                  activeAgent === index
                    ? "border-white bg-zinc-900"
                    : "border-zinc-800 bg-zinc-950 hover:border-zinc-700"
                }`}
                onClick={() => setActiveAgent(index)}
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center space-x-3">
                    <h3 className="text-xl font-semibold text-white">{kit.name}</h3>
                    <span className="px-2 py-1 bg-zinc-800 text-zinc-300 text-xs rounded font-mono">
                      {kit.platform}
                    </span>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation()
                      copyCommand(kit.command)
                    }}
                    className="h-8 px-2 text-zinc-400 hover:text-white"
                  >
                    {copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
                  </Button>
                </div>
                <p className="text-zinc-400 mb-4">{kit.description}</p>
                <code className="text-sm bg-zinc-800 px-3 py-2 rounded text-zinc-300 block">{kit.command}</code>
              </div>
            ))}
          </div>

          {/* Code Preview */}
          <div className="bg-zinc-900 border border-zinc-800 rounded-lg overflow-hidden">
            <div className="flex items-center justify-between px-4 py-3 bg-zinc-800 border-b border-zinc-700">
              <div className="flex items-center space-x-2">
                <div className="flex space-x-1.5">
                  <div className="w-3 h-3 rounded-full bg-red-500"></div>
                  <div className="w-3 h-3 rounded-full bg-yellow-500"></div>
                  <div className="w-3 h-3 rounded-full bg-green-500"></div>
                </div>
                <span className="text-sm text-zinc-400 ml-4">
                  {agentKits[activeAgent].name.toLowerCase().replace(" ", "-")}.js
                </span>
              </div>
            </div>
            <div className="p-6">
              <pre className="text-sm text-zinc-300 overflow-x-auto">
                <code>{agentKits[activeAgent].preview}</code>
              </pre>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
