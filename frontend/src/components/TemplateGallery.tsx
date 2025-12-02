import { useState, useEffect } from 'react';
import {
  Search,
  Folder,
  FileCode,
  Sparkles,
  Shield,
  Wallet,
  Building2,
  Users,
  Loader2,
  Plus,
} from 'lucide-react';
import { api } from '@/services/api';
import type { TemplateInfo } from '@/types/dsl';

interface TemplateGalleryProps {
  onSelectTemplate: (templateName: string) => void;
}

// Category icons mapping
const categoryIcons: Record<string, React.ComponentType<{ className?: string }>> = {
  'Identity Verification': Users,
  'Healthcare': Building2,
  'Supply Chain': Folder,
  'Financial': Wallet,
  'General': FileCode,
};

// Category colors
const categoryColors: Record<string, string> = {
  'Identity Verification': 'from-primary-500 to-primary-600',
  'Healthcare': 'from-accent-500 to-accent-600',
  'Supply Chain': 'from-warning-500 to-warning-600',
  'Financial': 'from-success-500 to-success-600',
  'General': 'from-dark-500 to-dark-600',
};

const categoryBadgeColors: Record<string, string> = {
  'Identity Verification': 'badge-primary',
  'Healthcare': 'badge-accent',
  'Supply Chain': 'badge-warning',
  'Financial': 'badge-success',
  'General': 'bg-dark-700/50 text-dark-300 border border-dark-600/50',
};

export function TemplateGallery({ onSelectTemplate }: TemplateGalleryProps) {
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [hoveredTemplate, setHoveredTemplate] = useState<string | null>(null);

  useEffect(() => {
    async function loadTemplates() {
      try {
        setLoading(true);
        const response = await api.listTemplates();
        setTemplates(response.templates);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load templates');
      } finally {
        setLoading(false);
      }
    }

    loadTemplates();
  }, []);

  // Group templates by category
  const groupedTemplates = templates.reduce((acc, template) => {
    const category = template.category || 'General';
    if (!acc[category]) {
      acc[category] = [];
    }
    acc[category].push(template);
    return acc;
  }, {} as Record<string, TemplateInfo[]>);

  // Get unique categories
  const categories = Object.keys(groupedTemplates);

  // Filter templates
  const filteredTemplates = templates.filter((template) => {
    const matchesSearch =
      searchQuery === '' ||
      template.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      template.description.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory =
      selectedCategory === null || template.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center">
          <Loader2 className="w-8 h-8 text-primary-500 animate-spin mx-auto mb-3" />
          <p className="text-dark-400 text-sm">Loading templates...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center p-4">
        <div className="text-center">
          <div className="w-12 h-12 rounded-xl bg-error-500/20 flex items-center justify-center mx-auto mb-3">
            <Shield className="w-6 h-6 text-error-400" />
          </div>
          <p className="text-error-400 text-sm mb-2">Failed to load templates</p>
          <p className="text-dark-500 text-xs">{error}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-dark-800/50">
        <div className="flex items-center space-x-2 mb-3">
          <Sparkles className="w-5 h-5 text-primary-400" />
          <h2 className="font-semibold text-white">Templates</h2>
          <span className="badge badge-primary text-xs">{templates.length}</span>
        </div>

        {/* Search */}
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-dark-500" />
          <input
            type="text"
            placeholder="Search templates..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="input pl-10 py-2 text-sm"
          />
        </div>
      </div>

      {/* Category Filter */}
      <div className="px-4 py-3 border-b border-dark-800/50">
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => setSelectedCategory(null)}
            className={`text-xs px-3 py-1.5 rounded-lg transition-all duration-200 ${
              selectedCategory === null
                ? 'bg-primary-500/20 text-primary-300 border border-primary-500/30'
                : 'bg-dark-800/50 text-dark-400 border border-dark-700/50 hover:bg-dark-700/50 hover:text-dark-300'
            }`}
          >
            All
          </button>
          {categories.map((category) => {
            const IconComponent = categoryIcons[category] || FileCode;
            return (
              <button
                key={category}
                onClick={() => setSelectedCategory(category)}
                className={`text-xs px-3 py-1.5 rounded-lg transition-all duration-200 flex items-center space-x-1.5 ${
                  selectedCategory === category
                    ? 'bg-primary-500/20 text-primary-300 border border-primary-500/30'
                    : 'bg-dark-800/50 text-dark-400 border border-dark-700/50 hover:bg-dark-700/50 hover:text-dark-300'
                }`}
              >
                <IconComponent className="w-3 h-3" />
                <span>{category}</span>
              </button>
            );
          })}
        </div>
      </div>

      {/* Template List */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {filteredTemplates.length === 0 ? (
          <div className="text-center py-8">
            <FileCode className="w-10 h-10 text-dark-600 mx-auto mb-3" />
            <p className="text-dark-500 text-sm">No templates found</p>
          </div>
        ) : (
          filteredTemplates.map((template) => {
            const IconComponent = categoryIcons[template.category] || FileCode;
            const gradientColor = categoryColors[template.category] || categoryColors['General'];
            const badgeColor = categoryBadgeColors[template.category] || categoryBadgeColors['General'];
            const isHovered = hoveredTemplate === template.name;

            return (
              <button
                key={template.name}
                onClick={() => onSelectTemplate(template.name)}
                onMouseEnter={() => setHoveredTemplate(template.name)}
                onMouseLeave={() => setHoveredTemplate(null)}
                className={`w-full text-left p-4 rounded-xl border transition-all duration-300 group ${
                  isHovered
                    ? 'bg-dark-800/80 border-primary-500/50 shadow-glow'
                    : 'bg-dark-800/30 border-dark-700/50 hover:bg-dark-800/50'
                }`}
              >
                <div className="flex items-start space-x-3">
                  <div
                    className={`w-10 h-10 rounded-xl bg-gradient-to-br ${gradientColor} flex items-center justify-center flex-shrink-0 transition-transform duration-300 ${
                      isHovered ? 'scale-110' : ''
                    }`}
                  >
                    <IconComponent className="w-5 h-5 text-white" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between mb-1">
                      <h3 className={`font-medium truncate transition-colors ${
                        isHovered ? 'text-primary-300' : 'text-white'
                      }`}>
                        {template.title}
                      </h3>
                    </div>
                    <p className="text-dark-400 text-sm line-clamp-2 mb-2">
                      {template.description}
                    </p>
                    <span className={`badge ${badgeColor} text-xs`}>
                      {template.category}
                    </span>
                  </div>
                </div>
              </button>
            );
          })
        )}
      </div>

      {/* Create Custom */}
      <div className="p-4 border-t border-dark-800/50">
        <button
          onClick={() => onSelectTemplate('')}
          className="w-full btn btn-secondary text-sm flex items-center justify-center"
        >
          <Plus className="w-4 h-4 mr-2" />
          Start from Scratch
        </button>
      </div>
    </div>
  );
}
