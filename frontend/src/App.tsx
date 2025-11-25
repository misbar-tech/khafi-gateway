import { useState } from 'react';
import { FileCode } from 'lucide-react';
import { TemplateGallery } from '@/components/TemplateGallery';
import { DslEditor } from '@/components/DslEditor';
import { LivePreview } from '@/components/LivePreview';
import { api } from '@/services/api';
import type { BusinessRulesDSL } from '@/types/dsl';

function App() {
  const [dsl, setDsl] = useState<BusinessRulesDSL | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSelectTemplate(templateName: string) {
    try {
      setLoading(true);
      const templateDsl = await api.getTemplate(templateName);
      setDsl(templateDsl);
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to load template');
    } finally {
      setLoading(false);
    }
  }

  function handleDslChange(newDsl: BusinessRulesDSL | null) {
    setDsl(newDsl);
  }

  return (
    <div className="h-screen flex flex-col bg-gray-50">
      {/* Header */}
      <header className="bg-white border-b border-gray-200 shadow-sm">
        <div className="px-6 py-4">
          <div className="flex items-center">
            <FileCode className="w-8 h-8 text-primary-600 mr-3" />
            <div>
              <h1 className="text-2xl font-bold text-gray-900">
                Khafi Logic Compiler
              </h1>
              <p className="text-sm text-gray-600">
                Create custom validation rules with zero-knowledge proofs
              </p>
            </div>
          </div>
        </div>
      </header>

      {/* Main 3-Panel Layout */}
      <main className="flex-1 overflow-hidden">
        <div className="h-full grid grid-cols-12 gap-4 p-4">
          {/* Left Panel: Template Gallery */}
          <div className="col-span-3 bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden">
            <TemplateGallery onSelectTemplate={handleSelectTemplate} />
          </div>

          {/* Center Panel: DSL Editor */}
          <div className="col-span-5 bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden">
            {loading ? (
              <div className="h-full flex items-center justify-center">
                <div className="text-center">
                  <div className="w-12 h-12 border-4 border-primary-600 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
                  <p className="text-gray-600">Loading template...</p>
                </div>
              </div>
            ) : (
              <DslEditor value={dsl} onChange={handleDslChange} />
            )}
          </div>

          {/* Right Panel: Live Preview */}
          <div className="col-span-4 rounded-lg shadow-sm border border-gray-200 overflow-hidden">
            <LivePreview dsl={dsl} />
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="bg-white border-t border-gray-200 px-6 py-3">
        <div className="flex items-center justify-between text-sm text-gray-600">
          <p>
            Powered by <span className="font-semibold">RISC Zero</span> zkVM
          </p>
          <p>
            Khafi Gateway v0.1.0
          </p>
        </div>
      </footer>
    </div>
  );
}

export default App;
