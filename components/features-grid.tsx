import Link from "next/link"
import { Terminal, Blocks, Zap, Code, GitBranch, Settings, Puzzle } from "lucide-react"

export function FeaturesGrid() {
  return (
    <section className="w-full py-16 md:py-24 bg-zinc-950 border-b border-zinc-800">
      <div className="container px-4 md:px-6">
        <div className="text-center mb-12">
          <h2 className="text-3xl md:text-4xl font-bold text-white mb-4">Everything you need to build AI agents</h2>
          <p className="text-zinc-400 text-lg max-w-2xl mx-auto">
            Pre-built components, rules, and behaviors that you can copy and paste into your projects.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-px border border-zinc-800 rounded-lg overflow-hidden">
          {/* Feature 1 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Terminal className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">CLI First</h3>
            <p className="text-zinc-400 text-sm mb-4">Install and manage agent components with simple CLI commands.</p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">npx agentkit add</code>
          </div>

          {/* Feature 2 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Blocks className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Component Library</h3>
            <p className="text-zinc-400 text-sm mb-4">Pre-built agent rules, modes, and behaviors ready to use.</p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">50+ components</code>
          </div>

          {/* Feature 3 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Code className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Copy & Paste</h3>
            <p className="text-zinc-400 text-sm mb-4">
              Copy code directly into your project. No complex setup required.
            </p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">Zero config</code>
          </div>

          {/* Feature 4 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Settings className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Customizable</h3>
            <p className="text-zinc-400 text-sm mb-4">Modify and extend components to fit your specific needs.</p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">Fully flexible</code>
          </div>

          {/* Feature 5 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Zap className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Fast Setup</h3>
            <p className="text-zinc-400 text-sm mb-4">Get your AI agent running in minutes, not hours.</p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">{"< 5 minutes"}</code>
          </div>

          {/* Feature 6 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <GitBranch className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Version Control</h3>
            <p className="text-zinc-400 text-sm mb-4">Track changes and manage different agent configurations.</p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">Git friendly</code>
          </div>

          {/* Feature 7 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Puzzle className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Modular Design</h3>
            <p className="text-zinc-400 text-sm mb-4">Mix and match components to create unique agent behaviors.</p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">Composable</code>
          </div>

          {/* CTA */}
          <div className="bg-zinc-900 p-6 flex flex-col justify-center hover:bg-zinc-800 transition-colors">
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white">Ready to build your first agent?</h3>
              <p className="text-zinc-400 text-sm">Start with our getting started guide</p>
              <Link
                href="https://docs.agents.ignitr.dev"
                className="inline-block px-4 py-2 bg-white text-black rounded-md hover:bg-zinc-200 transition-colors text-sm font-medium"
              >
                Get Started
              </Link>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
