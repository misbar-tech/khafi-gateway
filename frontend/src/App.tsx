import { useState } from 'react';
import { Shield, ArrowLeft, Loader2 } from 'lucide-react';
import { LandingPage } from '@/components/LandingPage';
import { TemplateGallery } from '@/components/TemplateGallery';
import { DslEditor } from '@/components/DslEditor';
import { LivePreview } from '@/components/LivePreview';
import { api } from '@/services/api';
import type { BusinessRulesDSL } from '@/types/dsl';

type View = 'landing' | 'app';

function App() {
  const [view, setView] = useState<View>('landing');
  const [dsl, setDsl] = useState<BusinessRulesDSL | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSelectTemplate(templateName: string) {
    try {
      setLoading(true);
      const templateDsl = await api.getTemplate(templateName);
      setDsl(templateDsl);
    } catch (err) {
      console.error('Failed to load template:', err);
    } finally {
      setLoading(false);
    }
  }

  function handleDslChange(newDsl: BusinessRulesDSL | null) {
    setDsl(newDsl);
  }

  function handleGetStarted() {
    setView('app');
  }

  function handleBackToLanding() {
    setView('landing');
    setDsl(null);
  }

  // Landing Page
  if (view === 'landing') {
    return <LandingPage onGetStarted={handleGetStarted} />;
  }

  // Main Application
  return (
    <div className="h-screen flex flex-col">
      {/* Noise overlay */}
      <div className="noise" />

      {/* Header */}
      <header className="glass-dark border-b border-dark-800/30 relative z-10">
        <div className="px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center">
              <button
                onClick={handleBackToLanding}
                className="mr-4 p-2 rounded-lg bg-dark-800/50 text-dark-400 hover:text-white hover:bg-dark-700/50 transition-colors"
              >
                <ArrowLeft className="w-5 h-5" />
              </button>
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary-500 to-accent-500 flex items-center justify-center mr-3">
                <Shield className="w-5 h-5 text-white" />
              </div>
              <div>
                <h1 className="text-xl font-bold text-white">
                  Logic Compiler
                </h1>
                <p className="text-sm text-dark-400">
                  Design zero-knowledge validation rules
                </p>
              </div>
            </div>
            <div className="flex items-center space-x-4">
              <div className="flex items-center space-x-2 text-sm text-dark-500">
                <div className="status-dot status-dot-success" />
                <span>Connected</span>
              </div>
              <button className="btn btn-secondary text-sm">
                Documentation
              </button>
            </div>
          </div>
        </div>
      </header>

      {/* Main 3-Panel Layout */}
      <main className="flex-1 overflow-hidden relative z-0">
        <div className="h-full grid grid-cols-12 gap-4 p-4">
          {/* Left Panel: Template Gallery */}
          <div className="col-span-3 card overflow-hidden flex flex-col">
            <TemplateGallery onSelectTemplate={handleSelectTemplate} />
          </div>

          {/* Center Panel: DSL Editor */}
          <div className="col-span-5 card overflow-hidden flex flex-col">
            {loading ? (
              <div className="h-full flex items-center justify-center">
                <div className="text-center">
                  <Loader2 className="w-12 h-12 text-primary-500 animate-spin mx-auto mb-4" />
                  <p className="text-dark-400">Loading template...</p>
                </div>
              </div>
            ) : (
              <DslEditor value={dsl} onChange={handleDslChange} />
            )}
          </div>

          {/* Right Panel: Live Preview */}
          <div className="col-span-4 card overflow-hidden flex flex-col">
            <LivePreview dsl={dsl} />
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="glass-dark border-t border-dark-800/30 px-6 py-3 relative z-10">
        <div className="flex items-center justify-between text-sm">
          <div className="flex items-center space-x-2 text-dark-500">
            <span>Powered by</span>
            <span className="badge badge-primary">RISC Zero zkVM</span>
            <span className="text-dark-600">+</span>
            <span className="badge badge-accent">Zcash Orchard</span>
          </div>
          <p className="text-dark-600">
            Khafi Gateway v0.1.0
          </p>
        </div>
      </footer>
    </div>
  );
}

export default App;
