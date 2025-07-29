import Link from "next/link"
import { Github, Twitter } from "lucide-react"

export function Footer() {
  return (
    <footer className="w-full bg-black border-t border-zinc-800">
      <div className="container px-4 py-12 md:px-6">
        <div className="grid gap-8 md:grid-cols-4">
          <div>
            <div className="flex items-center space-x-2 mb-4">
              <div className="w-8 h-8 bg-white rounded-md flex items-center justify-center">
                <span className="text-black font-bold text-sm">AK</span>
              </div>
              <span className="text-xl font-bold text-white">AgentKit</span>
            </div>
            <p className="text-sm text-zinc-400">
              Build AI agents with reusable components. Open source and developer-first.
            </p>
          </div>
          <div>
            <h3 className="mb-4 text-sm font-semibold text-white">Documentation</h3>
            <ul className="space-y-2 text-sm">
              <li>
                <Link href="/docs/getting-started" className="text-zinc-400 transition-colors hover:text-white">
                  Getting Started
                </Link>
              </li>
              <li>
                <Link href="/docs/installation" className="text-zinc-400 transition-colors hover:text-white">
                  Installation
                </Link>
              </li>
              <li>
                <Link href="/docs/cli" className="text-zinc-400 transition-colors hover:text-white">
                  CLI Reference
                </Link>
              </li>
              <li>
                <Link href="/docs/api" className="text-zinc-400 transition-colors hover:text-white">
                  API Reference
                </Link>
              </li>
            </ul>
          </div>
          <div>
            <h3 className="mb-4 text-sm font-semibold text-white">Components</h3>
            <ul className="space-y-2 text-sm">
              <li>
                <Link href="/components/agents" className="text-zinc-400 transition-colors hover:text-white">
                  Agent Types
                </Link>
              </li>
              <li>
                <Link href="/components/rules" className="text-zinc-400 transition-colors hover:text-white">
                  Rules
                </Link>
              </li>
              <li>
                <Link href="/components/modes" className="text-zinc-400 transition-colors hover:text-white">
                  Modes
                </Link>
              </li>
              <li>
                <Link href="/components/behaviors" className="text-zinc-400 transition-colors hover:text-white">
                  Behaviors
                </Link>
              </li>
            </ul>
          </div>
          <div>
            <h3 className="mb-4 text-sm font-semibold text-white">Community</h3>
            <ul className="space-y-2 text-sm">
              <li>
                <Link href="/examples" className="text-zinc-400 transition-colors hover:text-white">
                  Examples
                </Link>
              </li>
              <li>
                <Link href="/blocks" className="text-zinc-400 transition-colors hover:text-white">
                  Blocks
                </Link>
              </li>
              <li>
                <Link href="/changelog" className="text-zinc-400 transition-colors hover:text-white">
                  Changelog
                </Link>
              </li>
              <li>
                <Link href="/contributing" className="text-zinc-400 transition-colors hover:text-white">
                  Contributing
                </Link>
              </li>
            </ul>
          </div>
        </div>
        <div className="mt-12 border-t border-zinc-800 pt-6">
          <div className="flex flex-col items-center justify-between gap-4 md:flex-row">
            <p className="text-sm text-zinc-400">
              © {new Date().getFullYear()} AgentKit. Open source under MIT License.
            </p>
            <div className="flex space-x-4">
              <Link
                href="https://github.com/agentkit/agentkit"
                className="text-zinc-400 transition-colors hover:text-white"
              >
                <span className="sr-only">GitHub</span>
                <Github className="h-5 w-5" />
              </Link>
              <Link href="https://twitter.com/agentkit" className="text-zinc-400 transition-colors hover:text-white">
                <span className="sr-only">Twitter</span>
                <Twitter className="h-5 w-5" />
              </Link>
            </div>
          </div>
        </div>
      </div>
    </footer>
  )
}
