"use client"

import { Navbar } from "@/components/navbar"
import { Footer } from "@/components/footer"
import { Copy, Check, Download, Star, FolderSyncIcon as Sync } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useState } from "react"

const agentKits = [
  {
    name: "Cursor Engineer",
    description: "Code review, debugging, and technical documentation optimized for Cursor IDE",
    command: "npx agentkit add cursor-engineer",
    version: "v1.2.0",
    downloads: "12.5k",
    stars: 245,
    platform: "Cursor",
    tags: ["code", "debugging", "cursor"],
    features: [
      "Cursor-specific code analysis",
      "IDE-integrated debugging prompts",
      "Technical documentation generation",
      "Best practices enforcement",
    ],
  },
  {
    name: "Claude Writer",
    description: "Professional writing and content creation prompts for Claude",
    command: "npx agentkit add claude-writer",
    version: "v1.1.0",
    downloads: "8.3k",
    stars: 189,
    platform: "Claude",
    tags: ["content", "writing", "claude"],
    features: [
      "Content generation prompts",
      "Style and tone optimization",
      "SEO-focused writing",
      "Multi-format output",
    ],
  },
  {
    name: "N8N Workflow Agent",
    description: "Automation and workflow management for n8n platform",
    command: "npx agentkit add n8n-workflow",
    version: "v1.0.5",
    downloads: "6.7k",
    stars: 156,
    platform: "n8n",
    tags: ["automation", "workflow", "n8n"],
    features: ["Workflow optimization", "Error handling patterns", "Trigger management", "Data transformation"],
  },
  {
    name: "CLINE QA Agent",
    description: "Automated QA and testing agent for CLINE environments",
    command: "npx agentkit add cline-qa",
    version: "v0.9.2",
    downloads: "4.2k",
    stars: 98,
    platform: "CLINE",
    tags: ["qa", "testing", "cline"],
    features: ["Test case generation", "Bug detection patterns", "Quality assurance rules", "Automated reporting"],
  },
  {
    name: "Kilo Code Assistant",
    description: "Code assistance and pair programming for Kilo Code platform",
    command: "npx agentkit add kilo-assistant",
    version: "v1.0.1",
    downloads: "5.1k",
    stars: 134,
    platform: "Kilo Code",
    tags: ["coding", "assistant", "kilo"],
    features: ["Pair programming support", "Code suggestions", "Refactoring guidance", "Architecture advice"],
  },
  {
    name: "Rovoo Research Kit",
    description: "Research and information gathering agent for Rovoo platform",
    command: "npx agentkit add rovoo-research",
    version: "v0.8.7",
    downloads: "3.9k",
    stars: 87,
    platform: "Rovoo",
    tags: ["research", "data", "rovoo"],
    features: ["Information synthesis", "Source verification", "Research methodology", "Citation management"],
  },
]

function KitCard({ kit }: { kit: (typeof agentKits)[0] }) {
  const [copied, setCopied] = useState(false)

  const copyCommand = async () => {
    try {
      await navigator.clipboard.writeText(kit.command)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (err) {
      console.error("Failed to copy: ", err)
    }
  }

  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-6 hover:border-zinc-700 transition-colors">
      <div className="flex items-start justify-between mb-4">
        <div>
          <div className="flex items-center space-x-3 mb-2">
            <h3 className="text-xl font-semibold text-white">{kit.name}</h3>
            <span className="px-2 py-1 bg-zinc-800 text-zinc-300 text-xs rounded font-mono">{kit.platform}</span>
          </div>
          <p className="text-zinc-400 text-sm">{kit.description}</p>
        </div>
        <div className="flex items-center space-x-4 text-sm text-zinc-400">
          <div className="flex items-center space-x-1">
            <Download className="w-4 h-4" />
            <span>{kit.downloads}</span>
          </div>
          <div className="flex items-center space-x-1">
            <Star className="w-4 h-4" />
            <span>{kit.stars}</span>
          </div>
        </div>
      </div>

      <div className="flex flex-wrap gap-2 mb-4">
        {kit.tags.map((tag) => (
          <span key={tag} className="px-2 py-1 bg-zinc-800 text-zinc-300 text-xs rounded">
            {tag}
          </span>
        ))}
        <span className="px-2 py-1 bg-zinc-800 text-zinc-300 text-xs rounded">{kit.version}</span>
      </div>

      <div className="mb-4">
        <h4 className="text-sm font-medium text-white mb-2">Features:</h4>
        <ul className="text-sm text-zinc-400 space-y-1">
          {kit.features.map((feature, index) => (
            <li key={index} className="flex items-center">
              <span className="mr-2">•</span>
              {feature}
            </li>
          ))}
        </ul>
      </div>

      <div className="bg-zinc-800 rounded-md p-3 mb-4">
        <div className="flex items-center justify-between">
          <code className="text-sm text-zinc-300">{kit.command}</code>
          <Button variant="ghost" size="sm" onClick={copyCommand} className="h-6 px-2 text-zinc-400 hover:text-white">
            {copied ? <Check className="w-3 h-3" /> : <Copy className="w-3 h-3" />}
          </Button>
        </div>
      </div>

      <div className="flex space-x-2">
        <Button size="sm" className="bg-white text-black hover:bg-zinc-200 flex-1">
          <Download className="w-4 h-4 mr-2" />
          Install Kit
        </Button>
        <Button size="sm" variant="outline" className="border-zinc-700 text-white hover:bg-zinc-800 bg-transparent">
          <Sync className="w-4 h-4" />
        </Button>
      </div>
    </div>
  )
}

export default function KitsPage() {
  return (
    <div className="flex min-h-screen flex-col">
      <Navbar />
      <main className="flex-1 bg-black">
        <section className="w-full py-16 md:py-24">
          <div className="container px-4 md:px-6">
            <div className="text-center mb-12">
              <h1 className="text-4xl md:text-5xl font-bold text-white mb-4">AgentKit Registry</h1>
              <p className="text-zinc-400 text-lg max-w-2xl mx-auto">
                Browse platform-specific agent kits from the community. Install, sync, and share agent logic across your
                development workflow.
              </p>
            </div>

            <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3 max-w-7xl mx-auto">
              {agentKits.map((kit) => (
                <KitCard key={kit.name} kit={kit} />
              ))}
            </div>

            <div className="text-center mt-12">
              <p className="text-zinc-400 mb-4">Need a custom kit for your platform or workflow?</p>
              <Button variant="outline" className="border-zinc-700 text-white hover:bg-zinc-800 bg-transparent" asChild>
                <a href="https://docs.agents.ignitr.dev">Create Your Own Kit</a>
              </Button>
            </div>
          </div>
        </section>
      </main>
      <Footer />
    </div>
  )
}
