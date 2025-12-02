import { useState, useEffect } from 'react';
import Editor from '@monaco-editor/react';
import { Code2, FileJson, AlertTriangle, Wand2 } from 'lucide-react';
import type { BusinessRulesDSL } from '@/types/dsl';

interface DslEditorProps {
  value: BusinessRulesDSL | null;
  onChange: (value: BusinessRulesDSL | null) => void;
  onValidate?: (isValid: boolean, error?: string) => void;
}

export function DslEditor({ value, onChange, onValidate }: DslEditorProps) {
  const [editorValue, setEditorValue] = useState('');
  const [parseError, setParseError] = useState<string | null>(null);

  // Update editor when value prop changes
  useEffect(() => {
    if (value) {
      setEditorValue(JSON.stringify(value, null, 2));
      setParseError(null);
    }
  }, [value]);

  function handleEditorChange(newValue: string | undefined) {
    if (!newValue) {
      setEditorValue('');
      onChange(null);
      setParseError(null);
      onValidate?.(false, 'DSL is empty');
      return;
    }

    setEditorValue(newValue);

    // Try to parse JSON
    try {
      const parsed = JSON.parse(newValue) as BusinessRulesDSL;
      setParseError(null);
      onChange(parsed);
      onValidate?.(true);
    } catch (err) {
      const error = err instanceof Error ? err.message : 'Invalid JSON';
      setParseError(error);
      onChange(null);
      onValidate?.(false, error);
    }
  }

  function handleFormatCode() {
    try {
      const parsed = JSON.parse(editorValue);
      const formatted = JSON.stringify(parsed, null, 2);
      setEditorValue(formatted);
      onChange(parsed);
      setParseError(null);
    } catch {
      // Already has parse error, ignore
    }
  }

  // Count lines for stats
  const lineCount = editorValue.split('\n').length;
  const charCount = editorValue.length;

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-dark-800/50">
        <div className="flex items-center space-x-2">
          <Code2 className="w-5 h-5 text-primary-400" />
          <h2 className="font-semibold text-white">DSL Editor</h2>
          {editorValue && !parseError && (
            <span className="badge badge-success text-xs">Valid JSON</span>
          )}
        </div>
        <div className="flex items-center space-x-2">
          <button
            onClick={handleFormatCode}
            className="btn btn-ghost text-xs flex items-center px-3 py-1.5"
            disabled={!!parseError || !editorValue}
          >
            <Wand2 className="w-3.5 h-3.5 mr-1.5" />
            Format
          </button>
          <button
            onClick={handleFormatCode}
            className="btn btn-secondary text-xs flex items-center px-3 py-1.5"
            disabled={!!parseError || !editorValue}
          >
            <FileJson className="w-3.5 h-3.5 mr-1.5" />
            Pretty Print
          </button>
        </div>
      </div>

      {/* Parse Error */}
      {parseError && (
        <div className="px-4 py-3 bg-error-500/10 border-b border-error-500/30">
          <div className="flex items-start space-x-2">
            <AlertTriangle className="w-4 h-4 text-error-400 mt-0.5 flex-shrink-0" />
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-error-400">JSON Parse Error</p>
              <p className="text-xs text-error-400/80 mt-0.5 font-mono truncate">
                {parseError}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Editor */}
      <div className="flex-1 overflow-hidden bg-dark-950">
        <Editor
          height="100%"
          defaultLanguage="json"
          value={editorValue}
          onChange={handleEditorChange}
          theme="vs-dark"
          options={{
            minimap: { enabled: false },
            fontSize: 13,
            fontFamily: 'JetBrains Mono, Fira Code, monospace',
            lineNumbers: 'on',
            scrollBeyondLastLine: false,
            automaticLayout: true,
            tabSize: 2,
            wordWrap: 'on',
            formatOnPaste: true,
            formatOnType: true,
            padding: { top: 16, bottom: 16 },
            lineNumbersMinChars: 3,
            glyphMargin: false,
            folding: true,
            renderLineHighlight: 'line',
            cursorBlinking: 'smooth',
            cursorSmoothCaretAnimation: 'on',
            smoothScrolling: true,
            bracketPairColorization: { enabled: true },
            guides: {
              bracketPairs: true,
              indentation: true,
            },
          }}
          beforeMount={(monaco) => {
            // Define custom dark theme
            monaco.editor.defineTheme('khafi-dark', {
              base: 'vs-dark',
              inherit: true,
              rules: [
                { token: 'string.key.json', foreground: '5eead4' },
                { token: 'string.value.json', foreground: 'f0abfc' },
                { token: 'number', foreground: 'fbbf24' },
                { token: 'keyword', foreground: '2dd4bf' },
              ],
              colors: {
                'editor.background': '#020617',
                'editor.foreground': '#e2e8f0',
                'editor.lineHighlightBackground': '#1e293b',
                'editor.selectionBackground': '#14b8a640',
                'editorCursor.foreground': '#14b8a6',
                'editorLineNumber.foreground': '#475569',
                'editorLineNumber.activeForeground': '#94a3b8',
                'editor.inactiveSelectionBackground': '#14b8a620',
                'editorBracketMatch.background': '#14b8a640',
                'editorBracketMatch.border': '#14b8a6',
              },
            });
          }}
          onMount={(_editor, monaco) => {
            // Use custom theme
            monaco.editor.setTheme('khafi-dark');
          }}
        />
      </div>

      {/* Footer Stats */}
      <div className="px-4 py-2 border-t border-dark-800/50 flex items-center justify-between text-xs text-dark-500">
        <div className="flex items-center space-x-4">
          <span>JSON</span>
          <span className="text-dark-600">|</span>
          <span>{lineCount} lines</span>
          <span className="text-dark-600">|</span>
          <span>{charCount} characters</span>
        </div>
        <div className="flex items-center space-x-2">
          <span className="text-dark-600">UTF-8</span>
          <span className="text-dark-600">|</span>
          <span className="text-dark-600">2 spaces</span>
        </div>
      </div>
    </div>
  );
}
