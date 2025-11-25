import { useState, useEffect } from 'react';
import Editor from '@monaco-editor/react';
import { Code, FileJson } from 'lucide-react';
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
    } catch (err) {
      // Already has parse error, ignore
    }
  }

  return (
    <div className="h-full flex flex-col">
      <div className="flex items-center justify-between p-4 border-b border-gray-200 bg-white">
        <div className="flex items-center">
          <Code className="w-5 h-5 text-primary-600 mr-2" />
          <h2 className="text-lg font-semibold text-gray-900">DSL Editor</h2>
        </div>
        <button
          onClick={handleFormatCode}
          className="btn btn-secondary text-sm flex items-center"
          disabled={!!parseError}
        >
          <FileJson className="w-4 h-4 mr-2" />
          Format
        </button>
      </div>

      {parseError && (
        <div className="px-4 py-2 bg-red-50 border-b border-red-200">
          <p className="text-sm text-red-700 flex items-center">
            <span className="font-medium mr-2">JSON Parse Error:</span>
            {parseError}
          </p>
        </div>
      )}

      <div className="flex-1 overflow-hidden">
        <Editor
          height="100%"
          defaultLanguage="json"
          value={editorValue}
          onChange={handleEditorChange}
          theme="vs-light"
          options={{
            minimap: { enabled: false },
            fontSize: 14,
            lineNumbers: 'on',
            scrollBeyondLastLine: false,
            automaticLayout: true,
            tabSize: 2,
            wordWrap: 'on',
            formatOnPaste: true,
            formatOnType: true,
          }}
        />
      </div>
    </div>
  );
}
