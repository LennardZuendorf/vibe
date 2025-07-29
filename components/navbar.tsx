"use client"

import { useState } from "react"
import Link from "next/link"
import { Menu, X, Github } from "lucide-react"

import { Button } from "@/components/ui/button"
import { ThemeToggle } from "@/components/theme-toggle"

export function Navbar() {
  const [isMenuOpen, setIsMenuOpen] = useState(false)

  return (
    <header className="sticky top-0 z-50 w-full border-b border-zinc-800 bg-black/95 backdrop-blur supports-[backdrop-filter]:bg-black/80">
      <div className="container flex h-16 items-center justify-between px-4 md:px-6">
        <div className="flex items-center gap-6">
          <Link href="/" className="flex items-center space-x-2">
            <div className="w-8 h-8 bg-white rounded-md flex items-center justify-center">
              <span className="text-black font-bold text-sm">AK</span>
            </div>
            <span className="text-xl font-bold text-white">AgentKit</span>
          </Link>
          <nav className="hidden gap-6 md:flex">
            <Link href="/docs" className="text-sm font-medium text-zinc-400 transition-colors hover:text-white">
              Documentation
            </Link>
            <Link href="/components" className="text-sm font-medium text-zinc-400 transition-colors hover:text-white">
              Components
            </Link>
            <Link href="/examples" className="text-sm font-medium text-zinc-400 transition-colors hover:text-white">
              Examples
            </Link>
            <Link href="/blocks" className="text-sm font-medium text-zinc-400 transition-colors hover:text-white">
              Blocks
            </Link>
          </nav>
        </div>
        <div className="hidden md:flex md:items-center md:gap-2">
          <ThemeToggle />
          <Button variant="ghost" size="icon" className="h-9 w-9 rounded-md text-zinc-400 hover:text-white" asChild>
            <Link href="https://github.com/agentkit/agentkit">
              <Github className="h-[1.2rem] w-[1.2rem]" />
              <span className="sr-only">GitHub</span>
            </Link>
          </Button>
        </div>
        <button
          className="block rounded-md p-2.5 text-zinc-400 transition-colors hover:text-white md:hidden"
          onClick={() => setIsMenuOpen(!isMenuOpen)}
        >
          <span className="sr-only">Toggle menu</span>
          {isMenuOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
        </button>
      </div>
      {isMenuOpen && (
        <div className="container border-t border-zinc-800 px-4 py-4 md:hidden">
          <nav className="flex flex-col space-y-3">
            <Link href="/docs" className="text-sm font-medium text-zinc-400 transition-colors hover:text-white">
              Documentation
            </Link>
            <Link href="/components" className="text-sm font-medium text-zinc-400 transition-colors hover:text-white">
              Components
            </Link>
            <Link href="/examples" className="text-sm font-medium text-zinc-400 transition-colors hover:text-white">
              Examples
            </Link>
            <Link href="/blocks" className="text-sm font-medium text-zinc-400 transition-colors hover:text-white">
              Blocks
            </Link>
          </nav>
        </div>
      )}
    </header>
  )
}
