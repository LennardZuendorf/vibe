import Link from "next/link"
import { Terminal, GitBranch, FolderSyncIcon as Sync, Package, Shield, Globe, Code } from "lucide-react"

export function FeaturesGrid() {
  return (
    <section className="w-full py-16 md:py-24 bg-zinc-950 border-b border-zinc-800">
      <div className="container px-4 md:px-6">
        <div className="text-center mb-12">
          <h2 className="text-3xl md:text-4xl font-bold text-white mb-4">Universal agent logic management</h2>
          <p className="text-zinc-400 text-lg max-w-3xl mx-auto">
            The only tool that provides registry-based syncing and sharing of AI agent rules across platforms and teams.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-px border border-zinc-800 rounded-lg overflow-hidden">
          {/* Feature 1 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Sync className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Cross-Platform Sync</h3>
            <p className="text-zinc-400 text-sm mb-4">
              Sync agent configurations between Cursor, Claude, and other platforms seamlessly.
            </p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">Universal format</code>
          </div>

          {/* Feature 2 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <GitBranch className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Git-Based Registry</h3>
            <p className="text-zinc-400 text-sm mb-4">
              Version control and distribute kits using familiar Git workflows and repositories.
            </p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">Any Git repo</code>
          </div>

          {/* Feature 3 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Terminal className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">CLI-First</h3>
            <p className="text-zinc-400 text-sm mb-4">
              Developer-focused command line interface for power users and automation.
            </p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">npx agentkit</code>
          </div>

          {/* Feature 4 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Package className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Modular Kits</h3>
            <p className="text-zinc-400 text-sm mb-4">
              Package agent rules, prompts, and configurations into reusable components.
            </p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">Composable</code>
          </div>

          {/* Feature 5 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Globe className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Open Registry</h3>
            <p className="text-zinc-400 text-sm mb-4">
              No curation or gatekeeping. Anyone can contribute and host kits.
            </p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">Community-driven</code>
          </div>

          {/* Feature 6 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Shield className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Safe Updates</h3>
            <p className="text-zinc-400 text-sm mb-4">
              Built-in versioning and registry standards for predictable updates.
            </p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">Semantic versioning</code>
          </div>

          {/* Feature 7 */}
          <div className="bg-zinc-900 p-6 hover:bg-zinc-800 transition-colors">
            <div className="bg-zinc-800 rounded-md w-10 h-10 flex items-center justify-center mb-4">
              <Code className="h-5 w-5 text-white" />
            </div>
            <h3 className="text-lg font-semibold mb-2 text-white">Node.js Native</h3>
            <p className="text-zinc-400 text-sm mb-4">
              Built specifically for Node-based AI agents and workflow automation.
            </p>
            <code className="text-xs bg-zinc-800 px-2 py-1 rounded text-zinc-300">Node ecosystem</code>
          </div>

          {/* CTA */}
          <div className="bg-zinc-900 p-6 flex flex-col justify-center hover:bg-zinc-800 transition-colors">
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white">Ready to standardize your agent workflow?</h3>
              <p className="text-zinc-400 text-sm">Start with our developer guide</p>
              <Link
                href="https://docs.agents.ignitr.dev"
                className="inline-block px-4 py-2 bg-white text-black rounded-md hover:bg-zinc-200 transition-colors text-sm font-medium"
              >
                Read Docs
              </Link>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
