"use client"

import { useState } from "react"
import Link from "next/link"
import { Copy, Check, Terminal, ArrowRight } from "lucide-react"

import { Button } from "@/components/ui/button"

export function HeroSection() {
  const [copied, setCopied] = useState(false)
  const command = "npx agentkit add engineer"

  const copyToClipboard = async () => {
    try {
      await navigator.clipboard.writeText(command)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (err) {
      console.error("Failed to copy: ", err)
    }
  }

  return (
    <section className="w-full py-16 md:py-24 lg:py-32 bg-black">
      <div className="container px-4 md:px-6">
        <div className="flex flex-col items-center space-y-8 text-center">
          <div className="space-y-4 max-w-3xl">
            <h1 className="text-4xl md:text-5xl lg:text-6xl font-bold tracking-tight text-white">
              Build AI agents with{" "}
              <span className="bg-gradient-to-r from-white to-zinc-400 bg-clip-text text-transparent">
                reusable components
              </span>
            </h1>
            <p className="text-lg md:text-xl text-zinc-400 max-w-[700px] mx-auto">
              A CLI and component library for rapidly building AI agents. Copy, paste, and customize agent rules, modes,
              and behaviors.
            </p>
          </div>

          {/* Terminal Display */}
          <div className="w-full max-w-2xl mx-auto">
            <div className="bg-zinc-900 border border-zinc-800 rounded-lg overflow-hidden shadow-2xl">
              {/* Terminal Header */}
              <div className="flex items-center justify-between px-4 py-3 bg-zinc-800 border-b border-zinc-700">
                <div className="flex items-center space-x-2">
                  <div className="flex space-x-1.5">
                    <div className="w-3 h-3 rounded-full bg-red-500"></div>
                    <div className="w-3 h-3 rounded-full bg-yellow-500"></div>
                    <div className="w-3 h-3 rounded-full bg-green-500"></div>
                  </div>
                  <div className="flex items-center space-x-2 ml-4">
                    <Terminal className="w-4 h-4 text-zinc-400" />
                    <span className="text-sm text-zinc-400">terminal</span>
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={copyToClipboard}
                  className="h-8 px-2 text-zinc-400 hover:text-white hover:bg-zinc-700"
                >
                  {copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
                </Button>
              </div>

              {/* Terminal Content */}
              <div className="p-6 font-mono text-sm">
                <div className="flex items-center space-x-2 mb-2">
                  <span className="text-green-400">$</span>
                  <span className="text-white">{command}</span>
                </div>
                <div className="text-zinc-400 space-y-1">
                  <div>✓ Installing engineer agent...</div>
                  <div>✓ Adding rules and behaviors...</div>
                  <div>✓ Setting up custom modes...</div>
                  <div className="text-green-400">✓ Engineer agent ready!</div>
                </div>
              </div>
            </div>
          </div>

          {/* CTA Buttons */}
          <div className="flex flex-col sm:flex-row gap-4">
            <Button size="lg" className="bg-white hover:bg-zinc-200 text-black" asChild>
              <Link href="/docs">
                Get Started <ArrowRight className="ml-2 h-4 w-4" />
              </Link>
            </Button>
            <Button
              size="lg"
              variant="outline"
              className="border-zinc-700 text-white hover:bg-zinc-800 bg-transparent"
              asChild
            >
              <Link href="/components">Browse Components</Link>
            </Button>
          </div>

          {/* Quick Stats */}
          <div className="grid grid-cols-3 gap-8 pt-8 border-t border-zinc-800 w-full max-w-md">
            <div className="text-center">
              <div className="text-2xl font-bold text-white">50+</div>
              <div className="text-sm text-zinc-400">Components</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-white">12</div>
              <div className="text-sm text-zinc-400">Agent Types</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-white">100%</div>
              <div className="text-sm text-zinc-400">Open Source</div>
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
