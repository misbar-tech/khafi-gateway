import { useState } from 'react';
import {
  Shield,
  Zap,
  Lock,
  Code2,
  Wallet,
  ChevronRight,
  CheckCircle2,
  ArrowRight,
  Sparkles,
  Cpu,
  Globe,
  Users,
} from 'lucide-react';

interface LandingPageProps {
  onGetStarted: () => void;
}

export function LandingPage({ onGetStarted }: LandingPageProps) {
  const [hoveredFeature, setHoveredFeature] = useState<number | null>(null);

  const features = [
    {
      icon: Shield,
      title: 'Zero-Knowledge Proofs',
      description: 'Verify user data without exposing sensitive information. Privacy-preserving by design.',
      gradient: 'from-primary-500 to-primary-600',
    },
    {
      icon: Lock,
      title: 'Privacy-First',
      description: 'Built on Zcash Orchard protocol for maximum transaction privacy and security.',
      gradient: 'from-accent-500 to-accent-600',
    },
    {
      icon: Zap,
      title: 'Instant Deployment',
      description: 'Deploy your custom validation logic in minutes. No blockchain expertise required.',
      gradient: 'from-warning-500 to-warning-600',
    },
    {
      icon: Code2,
      title: 'No-Code Builder',
      description: 'Visual DSL editor with templates. Build complex rules without writing code.',
      gradient: 'from-success-500 to-success-600',
    },
  ];

  const steps = [
    {
      number: '01',
      title: 'Define Your Rules',
      description: 'Use our visual editor to create validation logic for age checks, compliance, or custom requirements.',
    },
    {
      number: '02',
      title: 'Deploy to Gateway',
      description: 'One-click deployment compiles your rules into a ZK circuit and registers it on the network.',
    },
    {
      number: '03',
      title: 'Integrate & Verify',
      description: 'Use our API to verify proofs. Users prove compliance without revealing private data.',
    },
  ];

  const stats = [
    { value: '100%', label: 'Privacy Preserved' },
    { value: '<1s', label: 'Proof Generation' },
    { value: '99.9%', label: 'Uptime SLA' },
    { value: '0', label: 'Data Exposure' },
  ];

  return (
    <div className="min-h-screen">
      {/* Noise overlay */}
      <div className="noise" />

      {/* Navigation */}
      <nav className="fixed top-0 left-0 right-0 z-50 glass-dark border-b border-dark-800/30">
        <div className="max-w-7xl mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary-500 to-accent-500 flex items-center justify-center">
                <Shield className="w-5 h-5 text-white" />
              </div>
              <span className="text-xl font-bold gradient-text">Khafi</span>
            </div>
            <div className="hidden md:flex items-center space-x-8">
              <a href="#features" className="text-dark-400 hover:text-white transition-colors">Features</a>
              <a href="#how-it-works" className="text-dark-400 hover:text-white transition-colors">How it Works</a>
              <a href="#pricing" className="text-dark-400 hover:text-white transition-colors">Pricing</a>
              <a href="https://docs.khafi.io" className="text-dark-400 hover:text-white transition-colors">Docs</a>
            </div>
            <div className="flex items-center space-x-4">
              <button className="btn btn-ghost">Sign In</button>
              <button onClick={onGetStarted} className="btn btn-primary">
                Get Started
                <ChevronRight className="w-4 h-4 ml-1" />
              </button>
            </div>
          </div>
        </div>
      </nav>

      {/* Hero Section */}
      <section className="relative pt-32 pb-20 overflow-hidden">
        {/* Background effects */}
        <div className="absolute inset-0 overflow-hidden">
          <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-primary-500/20 rounded-full blur-[128px] animate-pulse-slow" />
          <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-accent-500/20 rounded-full blur-[128px] animate-pulse-slow" style={{ animationDelay: '-2s' }} />
        </div>

        <div className="relative max-w-7xl mx-auto px-6">
          <div className="text-center max-w-4xl mx-auto">
            {/* Badge */}
            <div className="inline-flex items-center space-x-2 badge badge-primary mb-8 animate-fade-in">
              <Sparkles className="w-3.5 h-3.5" />
              <span>Powered by RISC Zero zkVM</span>
            </div>

            {/* Headline */}
            <h1 className="text-5xl md:text-7xl font-bold mb-6 animate-slide-up">
              <span className="text-white">Privacy-First</span>
              <br />
              <span className="gradient-text">API Gateway</span>
            </h1>

            <p className="text-xl text-dark-400 mb-10 max-w-2xl mx-auto animate-slide-up" style={{ animationDelay: '0.1s' }}>
              Deploy custom zero-knowledge verification logic in minutes.
              Prove user compliance without exposing sensitive data. Built on Zcash for maximum privacy.
            </p>

            {/* CTAs */}
            <div className="flex flex-col sm:flex-row items-center justify-center gap-4 animate-slide-up" style={{ animationDelay: '0.2s' }}>
              <button onClick={onGetStarted} className="btn btn-primary text-lg px-8 py-4">
                Start Building
                <ArrowRight className="w-5 h-5 ml-2" />
              </button>
              <button className="btn btn-secondary text-lg px-8 py-4">
                View Documentation
              </button>
            </div>

            {/* Stats */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-6 mt-20 animate-fade-in" style={{ animationDelay: '0.4s' }}>
              {stats.map((stat, index) => (
                <div key={index} className="text-center">
                  <div className="text-3xl md:text-4xl font-bold gradient-text mb-1">{stat.value}</div>
                  <div className="text-sm text-dark-500">{stat.label}</div>
                </div>
              ))}
            </div>
          </div>

          {/* Hero visual */}
          <div className="mt-20 relative">
            <div className="absolute inset-0 bg-gradient-to-t from-dark-950 via-transparent to-transparent z-10" />
            <div className="card p-1 overflow-hidden animate-scale-in" style={{ animationDelay: '0.3s' }}>
              <div className="bg-dark-950 rounded-xl p-6">
                {/* Mock UI preview */}
                <div className="flex items-center space-x-2 mb-4">
                  <div className="w-3 h-3 rounded-full bg-error-500" />
                  <div className="w-3 h-3 rounded-full bg-warning-500" />
                  <div className="w-3 h-3 rounded-full bg-success-500" />
                  <span className="ml-4 text-dark-500 text-sm font-mono">khafi-gateway — DSL Editor</span>
                </div>
                <div className="grid grid-cols-3 gap-4 h-64">
                  <div className="bg-dark-900/50 rounded-lg p-4 border border-dark-800/50">
                    <div className="text-xs text-dark-500 mb-2">Templates</div>
                    <div className="space-y-2">
                      <div className="bg-primary-500/20 text-primary-300 px-3 py-2 rounded-lg text-sm">Age Verification</div>
                      <div className="bg-dark-800/50 text-dark-400 px-3 py-2 rounded-lg text-sm">KYC Compliance</div>
                      <div className="bg-dark-800/50 text-dark-400 px-3 py-2 rounded-lg text-sm">Access Control</div>
                    </div>
                  </div>
                  <div className="bg-dark-900/50 rounded-lg p-4 border border-dark-800/50 font-mono text-xs">
                    <div className="text-dark-500 mb-2">// DSL Editor</div>
                    <div className="text-accent-400">{"{"}</div>
                    <div className="text-dark-300 ml-4">"use_case": <span className="text-primary-400">"age_verification"</span>,</div>
                    <div className="text-dark-300 ml-4">"rules": [</div>
                    <div className="text-dark-300 ml-8">{"{ \"type\": \"age_check\" }"}</div>
                    <div className="text-dark-300 ml-4">]</div>
                    <div className="text-accent-400">{"}"}</div>
                  </div>
                  <div className="bg-dark-900/50 rounded-lg p-4 border border-dark-800/50">
                    <div className="text-xs text-dark-500 mb-2">Deploy Status</div>
                    <div className="flex items-center text-success-400 mb-4">
                      <CheckCircle2 className="w-4 h-4 mr-2" />
                      <span className="text-sm">Ready to deploy</span>
                    </div>
                    <button className="w-full btn btn-primary text-sm">
                      Deploy to Gateway
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section id="features" className="py-24 relative">
        <div className="max-w-7xl mx-auto px-6">
          <div className="text-center mb-16">
            <div className="badge badge-accent mb-4">Features</div>
            <h2 className="text-4xl md:text-5xl font-bold text-white mb-4">
              Built for Privacy
            </h2>
            <p className="text-dark-400 text-lg max-w-2xl mx-auto">
              Everything you need to implement zero-knowledge verification in your application.
            </p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
            {features.map((feature, index) => (
              <div
                key={index}
                className="card card-hover card-glow p-6 cursor-pointer"
                onMouseEnter={() => setHoveredFeature(index)}
                onMouseLeave={() => setHoveredFeature(null)}
              >
                <div className={`w-12 h-12 rounded-xl bg-gradient-to-br ${feature.gradient} flex items-center justify-center mb-4 transition-transform duration-300 ${hoveredFeature === index ? 'scale-110' : ''}`}>
                  <feature.icon className="w-6 h-6 text-white" />
                </div>
                <h3 className="text-lg font-semibold text-white mb-2">{feature.title}</h3>
                <p className="text-dark-400 text-sm">{feature.description}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* How it Works */}
      <section id="how-it-works" className="py-24 relative">
        <div className="absolute inset-0 bg-dark-900/30" />
        <div className="relative max-w-7xl mx-auto px-6">
          <div className="text-center mb-16">
            <div className="badge badge-primary mb-4">How it Works</div>
            <h2 className="text-4xl md:text-5xl font-bold text-white mb-4">
              Three Simple Steps
            </h2>
            <p className="text-dark-400 text-lg max-w-2xl mx-auto">
              From idea to production in minutes, not months.
            </p>
          </div>

          <div className="grid md:grid-cols-3 gap-8">
            {steps.map((step, index) => (
              <div key={index} className="relative">
                {index < steps.length - 1 && (
                  <div className="hidden md:block absolute top-12 left-1/2 w-full h-px bg-gradient-to-r from-primary-500/50 to-transparent" />
                )}
                <div className="card p-8 text-center relative z-10">
                  <div className="text-5xl font-bold gradient-text mb-4">{step.number}</div>
                  <h3 className="text-xl font-semibold text-white mb-3">{step.title}</h3>
                  <p className="text-dark-400">{step.description}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Use Cases */}
      <section className="py-24">
        <div className="max-w-7xl mx-auto px-6">
          <div className="text-center mb-16">
            <div className="badge badge-accent mb-4">Use Cases</div>
            <h2 className="text-4xl md:text-5xl font-bold text-white mb-4">
              Endless Possibilities
            </h2>
          </div>

          <div className="grid md:grid-cols-3 gap-6">
            <div className="card p-6 border-primary-500/30 hover:border-primary-500/50 transition-colors">
              <Users className="w-10 h-10 text-primary-400 mb-4" />
              <h3 className="text-lg font-semibold text-white mb-2">Age Verification</h3>
              <p className="text-dark-400 text-sm mb-4">
                Verify users are above age thresholds without collecting birthdates.
              </p>
              <div className="flex flex-wrap gap-2">
                <span className="badge badge-primary text-xs">Gaming</span>
                <span className="badge badge-primary text-xs">Alcohol</span>
                <span className="badge badge-primary text-xs">Adult Content</span>
              </div>
            </div>

            <div className="card p-6 border-accent-500/30 hover:border-accent-500/50 transition-colors">
              <Wallet className="w-10 h-10 text-accent-400 mb-4" />
              <h3 className="text-lg font-semibold text-white mb-2">Financial Compliance</h3>
              <p className="text-dark-400 text-sm mb-4">
                Prove income ranges, credit scores, or AML compliance privately.
              </p>
              <div className="flex flex-wrap gap-2">
                <span className="badge badge-accent text-xs">DeFi</span>
                <span className="badge badge-accent text-xs">Lending</span>
                <span className="badge badge-accent text-xs">KYC</span>
              </div>
            </div>

            <div className="card p-6 border-success-500/30 hover:border-success-500/50 transition-colors">
              <Globe className="w-10 h-10 text-success-400 mb-4" />
              <h3 className="text-lg font-semibold text-white mb-2">Access Control</h3>
              <p className="text-dark-400 text-sm mb-4">
                Gate access based on membership, location, or custom criteria.
              </p>
              <div className="flex flex-wrap gap-2">
                <span className="badge badge-success text-xs">DAOs</span>
                <span className="badge badge-success text-xs">Subscription</span>
                <span className="badge badge-success text-xs">Geo-fencing</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="py-24 relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-b from-dark-900/50 to-dark-950" />
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[800px] bg-primary-500/10 rounded-full blur-[128px]" />

        <div className="relative max-w-4xl mx-auto px-6 text-center">
          <h2 className="text-4xl md:text-5xl font-bold text-white mb-6">
            Ready to Build with Privacy?
          </h2>
          <p className="text-xl text-dark-400 mb-10">
            Join developers building the next generation of privacy-preserving applications.
          </p>
          <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
            <button onClick={onGetStarted} className="btn btn-primary text-lg px-10 py-4 pulse-glow">
              Start Building Free
              <Cpu className="w-5 h-5 ml-2" />
            </button>
            <button className="btn btn-outline text-lg px-10 py-4">
              Talk to Sales
            </button>
          </div>
          <p className="text-dark-500 text-sm mt-6">
            No credit card required. Free tier includes 1,000 proof generations/month.
          </p>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-dark-800/50 py-12">
        <div className="max-w-7xl mx-auto px-6">
          <div className="flex flex-col md:flex-row items-center justify-between">
            <div className="flex items-center space-x-2 mb-4 md:mb-0">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-primary-500 to-accent-500 flex items-center justify-center">
                <Shield className="w-4 h-4 text-white" />
              </div>
              <span className="text-lg font-bold gradient-text">Khafi Gateway</span>
            </div>
            <div className="flex items-center space-x-8 text-dark-500 text-sm">
              <a href="#" className="hover:text-white transition-colors">Privacy</a>
              <a href="#" className="hover:text-white transition-colors">Terms</a>
              <a href="#" className="hover:text-white transition-colors">Docs</a>
              <a href="#" className="hover:text-white transition-colors">GitHub</a>
            </div>
            <div className="text-dark-600 text-sm mt-4 md:mt-0">
              © 2024 Khafi. All rights reserved.
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
}
