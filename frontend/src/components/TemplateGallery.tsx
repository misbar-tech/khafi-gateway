import { useState, useEffect } from 'react';
import { FileCode, Loader2, AlertCircle } from 'lucide-react';
import { api } from '@/services/api';
import type { TemplateInfo } from '@/types/dsl';

interface TemplateGalleryProps {
  onSelectTemplate: (templateName: string) => void;
}

export function TemplateGallery({ onSelectTemplate }: TemplateGalleryProps) {
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(null);

  useEffect(() => {
    loadTemplates();
  }, []);

  async function loadTemplates() {
    try {
      setLoading(true);
      setError(null);
      const response = await api.listTemplates();
      setTemplates(response.templates);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load templates');
    } finally {
      setLoading(false);
    }
  }

  function handleSelectTemplate(templateName: string) {
    setSelectedTemplate(templateName);
    onSelectTemplate(templateName);
  }

  // Group templates by category
  const categorizedTemplates = templates.reduce((acc, template) => {
    const category = template.category || 'General';
    if (!acc[category]) {
      acc[category] = [];
    }
    acc[category].push(template);
    return acc;
  }, {} as Record<string, TemplateInfo[]>);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="w-8 h-8 animate-spin text-primary-600" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-4 text-center">
        <AlertCircle className="w-12 h-12 text-red-500 mb-4" />
        <p className="text-gray-600 mb-4">{error}</p>
        <button onClick={loadTemplates} className="btn btn-primary">
          Retry
        </button>
      </div>
    );
  }

  if (templates.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-4 text-center">
        <FileCode className="w-12 h-12 text-gray-400 mb-4" />
        <p className="text-gray-600">No templates available</p>
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto">
      <div className="p-4">
        <div className="mb-6">
          <h2 className="text-xl font-bold text-gray-900 mb-2">
            Template Gallery
          </h2>
          <p className="text-sm text-gray-600">
            Choose a template to get started quickly
          </p>
        </div>

        {Object.entries(categorizedTemplates).map(([category, categoryTemplates]) => (
          <div key={category} className="mb-6">
            <h3 className="text-sm font-semibold text-gray-700 uppercase tracking-wider mb-3">
              {category}
            </h3>
            <div className="space-y-2">
              {categoryTemplates.map((template) => (
                <button
                  key={template.name}
                  onClick={() => handleSelectTemplate(template.name)}
                  className={`w-full text-left card p-4 hover:shadow-md transition-all duration-200 ${
                    selectedTemplate === template.name
                      ? 'ring-2 ring-primary-500 bg-primary-50'
                      : ''
                  }`}
                >
                  <div className="flex items-start">
                    <FileCode className="w-5 h-5 text-primary-600 mt-1 mr-3 flex-shrink-0" />
                    <div className="flex-1 min-w-0">
                      <h4 className="font-medium text-gray-900 truncate mb-1">
                        {template.title}
                      </h4>
                      <p className="text-sm text-gray-600 line-clamp-2">
                        {template.description}
                      </p>
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
