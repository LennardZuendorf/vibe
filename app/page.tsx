import { HeroSection } from "@/components/hero-section"
import { FeaturesGrid } from "@/components/features-grid"
import { AgentShowcase } from "@/components/agent-showcase"
import { InstallationSection } from "@/components/installation-section"
import { Navbar } from "@/components/navbar"
import { Footer } from "@/components/footer"

export default function Home() {
  return (
    <div className="flex min-h-screen flex-col">
      <Navbar />
      <main className="flex-1">
        <HeroSection />
        <FeaturesGrid />
        <AgentShowcase />
        <InstallationSection />
      </main>
      <Footer />
    </div>
  )
}
