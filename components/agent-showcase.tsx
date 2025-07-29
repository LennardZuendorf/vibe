"use client"

import { useState } from "react"
import { Copy, Check } from "lucide-react"
import { Button } from "@/components/ui/button"

const agents = [
  {
    name: "Engineer",
    description: "Code review, debugging, and technical documentation",
    command: "npx agentkit add engineer",
    preview: `// Engineer Agent Configuration
export const engineerAgent = {
  name: "Engineer",
  rules: [
    "Always provide code examples",
    "Explain technical concepts clearly",
    "Suggest best practices"
  ],
  modes: ["debug", "review", "document"],
  capabilities: ["code-analysis", "testing", "optimization"]
}`,
  },
  {
    name: "Writer",
    description: "Content creation, editing, and style optimization",
    command: "npx agentkit add writer",
    preview: `// Writer Agent Configuration
export const writerAgent = {
  name: "Writer",
  rules: [
    "Maintain consistent tone",
    "Check grammar and style",
    "Optimize for readability"
  ],
  modes: ["creative", "technical", "marketing"],
  capabilities: ["editing", "proofreading", "seo"]
}`,
  },
  {
    name: "Analyst",
    description: "Data analysis, insights, and reporting",
    command: "npx agentkit add analyst",
    preview: `// Analyst Agent Configuration
export const analystAgent = {
  name: "Analyst",
  rules: [
    "Support claims with data",
    "Provide actionable insights",
    "Use clear visualizations"
  ],
  modes: ["research", "report", "forecast"],
  capabilities: ["data-viz", "statistics", "trends"]
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
          <h2 className="text-3xl md:text-4xl font-bold text-white mb-4">Popular Agent Types</h2>
          <p className="text-zinc-400 text-lg max-w-2xl mx-auto">
            Choose from our collection of pre-configured agents or use them as starting points for your custom
            implementations.
          </p>
        </div>

        <div className="grid lg:grid-cols-2 gap-8 max-w-6xl mx-auto">
          {/* Agent Selector */}
          <div className="space-y-4">
            {agents.map((agent, index) => (
              <div
                key={agent.name}
                className={`p-6 rounded-lg border cursor-pointer transition-all ${
                  activeAgent === index
                    ? "border-white bg-zinc-900"
                    : "border-zinc-800 bg-zinc-950 hover:border-zinc-700"
                }`}
                onClick={() => setActiveAgent(index)}
              >
                <div className="flex items-center justify-between mb-2">
                  <h3 className="text-xl font-semibold text-white">{agent.name}</h3>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation()
                      copyCommand(agent.command)
                    }}
                    className="h-8 px-2 text-zinc-400 hover:text-white"
                  >
                    {copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
                  </Button>
                </div>
                <p className="text-zinc-400 mb-4">{agent.description}</p>
                <code className="text-sm bg-zinc-800 px-3 py-2 rounded text-zinc-300 block">{agent.command}</code>
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
                <span className="text-sm text-zinc-400 ml-4">{agents[activeAgent].name.toLowerCase()}-agent.js</span>
              </div>
            </div>
            <div className="p-6">
              <pre className="text-sm text-zinc-300 overflow-x-auto">
                <code>{agents[activeAgent].preview}</code>
              </pre>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
