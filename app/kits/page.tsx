"use client"

import { Navbar } from "@/components/navbar"
import { Footer } from "@/components/footer"
import { Copy, Check, Download, Star, GitBranch } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useState } from "react"

const agentKits = [
  {
    name: "Engineer Kit",
    description: "Complete engineering agent with code review, debugging, and documentation capabilities",
    command: "npx agentkit add engineer",
    version: "v1.2.0",
    downloads: "12.5k",
    stars: 245,
    tags: ["code", "debugging", "documentation"],
    features: [
      "Code review and analysis",
      "Bug detection and fixes",
      "Technical documentation",
      "Best practices suggestions",
    ],
  },
  {
    name: "Writer Kit",
    description: "Professional writing agent for content creation, editing, and optimization",
    command: "npx agentkit add writer",
    version: "v1.1.0",
    downloads: "8.3k",
    stars: 189,
    tags: ["content", "editing", "seo"],
    features: ["Content generation", "Grammar and style checking", "SEO optimization", "Multiple writing styles"],
  },
  {
    name: "Analyst Kit",
    description: "Data analysis agent with reporting, insights, and visualization capabilities",
    command: "npx agentkit add analyst",
    version: "v1.0.5",
    downloads: "6.7k",
    stars: 156,
    tags: ["data", "analytics", "reporting"],
    features: ["Data analysis and insights", "Report generation", "Trend identification", "Statistical analysis"],
  },
  {
    name: "Support Kit",
    description: "Customer support agent with ticket handling and knowledge base integration",
    command: "npx agentkit add support",
    version: "v0.9.2",
    downloads: "4.2k",
    stars: 98,
    tags: ["support", "tickets", "knowledge"],
    features: ["Ticket classification", "Response generation", "Knowledge base search", "Escalation handling"],
  },
  {
    name: "Marketing Kit",
    description: "Marketing agent for campaign creation, social media, and content strategy",
    command: "npx agentkit add marketing",
    version: "v1.0.1",
    downloads: "5.1k",
    stars: 134,
    tags: ["marketing", "social", "campaigns"],
    features: ["Campaign planning", "Social media content", "A/B testing suggestions", "Performance tracking"],
  },
  {
    name: "Research Kit",
    description: "Research agent for information gathering, synthesis, and fact-checking",
    command: "npx agentkit add research",
    version: "v0.8.7",
    downloads: "3.9k",
    stars: 87,
    tags: ["research", "facts", "synthesis"],
    features: ["Information gathering", "Source verification", "Content synthesis", "Citation management"],
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
          <h3 className="text-xl font-semibold text-white mb-2">{kit.name}</h3>
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
          Install Kit
        </Button>
        <Button size="sm" variant="outline" className="border-zinc-700 text-white hover:bg-zinc-800 bg-transparent">
          <GitBranch className="w-4 h-4" />
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
              <h1 className="text-4xl md:text-5xl font-bold text-white mb-4">AgentKits</h1>
              <p className="text-zinc-400 text-lg max-w-2xl mx-auto">
                Browse and install pre-built agent kits for your projects. Each kit includes rules, behaviors, and modes
                tailored for specific use cases.
              </p>
            </div>

            <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3 max-w-7xl mx-auto">
              {agentKits.map((kit) => (
                <KitCard key={kit.name} kit={kit} />
              ))}
            </div>

            <div className="text-center mt-12">
              <p className="text-zinc-400 mb-4">Can't find what you're looking for?</p>
              <Button variant="outline" className="border-zinc-700 text-white hover:bg-zinc-800 bg-transparent" asChild>
                <a href="https://docs.agents.ignitr.dev">Create Custom Kit</a>
              </Button>
            </div>
          </div>
        </section>
      </main>
      <Footer />
    </div>
  )
}
