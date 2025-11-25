# Khafi Logic Compiler UI

Modern React + TypeScript frontend for creating and deploying custom business logic validation rules.

## Features

- **3-Panel Layout**
  - Template Gallery: Browse pre-built validation templates
  - DSL Editor: Edit JSON DSL with Monaco Editor
  - Live Preview: Real-time validation and code generation

- **Template Management**
  - Categorized templates (Healthcare, Supply Chain, Identity, etc.)
  - One-click template loading
  - Visual template cards with descriptions

- **DSL Editor**
  - Monaco Editor with JSON syntax highlighting
  - Real-time JSON validation
  - Auto-formatting
  - Error highlighting

- **Live Preview & Actions**
  - Real-time DSL validation
  - Code compilation preview
  - SDK generation and download
  - Visual status indicators

## Tech Stack

- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tool and dev server
- **Tailwind CSS** - Styling
- **Monaco Editor** - Code editor
- **Lucide React** - Icons

## Quick Start

### Prerequisites

- Node.js 18+ and npm
- Running Logic Compiler API (on port 8082)

### Installation

```bash
cd frontend
npm install
```

### Development

```bash
# Start dev server
npm run dev

# Open browser to http://localhost:3000
```

### Build for Production

```bash
npm run build

# Preview production build
npm run preview
```

## Project Structure

```
frontend/
├── src/
│   ├── components/
│   │   ├── TemplateGallery.tsx   # Template browser
│   │   ├── DslEditor.tsx          # Monaco editor wrapper
│   │   └── LivePreview.tsx        # Validation & actions
│   ├── services/
│   │   └── api.ts                 # API client
│   ├── types/
│   │   └── dsl.ts                 # TypeScript types
│   ├── App.tsx                    # Main layout
│   ├── main.tsx                   # Entry point
│   └── index.css                  # Global styles
├── package.json
├── vite.config.ts
├── tailwind.config.js
└── tsconfig.json
```

## Configuration

Create a `.env` file (or use `.env.example` as template):

```bash
VITE_API_URL=http://localhost:8082
```

## Usage

### 1. Select a Template

Click on any template in the left panel to load it into the editor.

### 2. Edit DSL

Modify the JSON DSL in the center panel. The editor provides:
- Syntax highlighting
- Auto-completion
- Real-time error detection
- Format button

### 3. Validate

The DSL is automatically validated as you type. Status appears in the right panel:
- ✅ Valid - Ready to compile
- ❌ Invalid - Shows error details

### 4. Compile & Deploy

Once valid, you can:
- **Compile to Code** - See generated Rust guest program
- **Generate & Download SDK** - Get complete SDK package

## API Integration

The frontend communicates with the Logic Compiler API:

- `GET /api/templates` - List templates
- `GET /api/templates/{name}` - Get template
- `POST /api/validate` - Validate DSL
- `POST /api/compile` - Compile to code
- `POST /api/sdk/generate` - Generate SDK
- `GET /api/sdk/download/{id}` - Download SDK

See `src/services/api.ts` for implementation details.

## Component Details

### TemplateGallery

Displays categorized validation templates with search and filtering.

**Props:**
- `onSelectTemplate: (name: string) => void` - Callback when template selected

**Features:**
- Grouped by category
- Visual selection indicator
- Loading and error states
- Empty state handling

### DslEditor

Monaco Editor wrapper for editing DSL JSON.

**Props:**
- `value: BusinessRulesDSL | null` - Current DSL
- `onChange: (dsl: BusinessRulesDSL | null) => void` - Change handler
- `onValidate?: (valid: boolean, error?: string) => void` - Validation callback

**Features:**
- Real-time JSON parsing
- Format button
- Parse error display
- Syntax highlighting

### LivePreview

Shows validation status, compiled code, and action buttons.

**Props:**
- `dsl: BusinessRulesDSL | null` - DSL to preview
- `autoValidate?: boolean` - Auto-validate on change (default: true)

**Features:**
- Real-time validation
- Status indicators
- Compile button
- SDK generation and download
- Code preview toggle

## Styling

Uses Tailwind CSS with custom theme:

```javascript
// Primary color palette
primary: {
  50: '#f0f9ff',
  500: '#0ea5e9',
  600: '#0284c7',
  700: '#0369a1',
}
```

Custom components defined in `index.css`:
- `.btn` - Button base
- `.btn-primary` - Primary button
- `.btn-secondary` - Secondary button
- `.btn-success` - Success button
- `.card` - Card container
- `.input` - Form input

## Development Tips

### Hot Module Replacement

Vite provides instant HMR. Changes appear immediately without refresh.

### Type Checking

Run TypeScript compiler in watch mode:

```bash
npm run tsc -- --watch
```

### Linting

```bash
npm run lint
```

### Monaco Editor Configuration

Editor options configured in `DslEditor.tsx`:

```typescript
options={{
  minimap: { enabled: false },
  fontSize: 14,
  lineNumbers: 'on',
  wordWrap: 'on',
  formatOnPaste: true,
  formatOnType: true,
}}
```

## Troubleshooting

### API Connection Issues

**Problem:** "Network error" when loading templates

**Solution:**
1. Ensure Logic Compiler API is running on port 8082
2. Check VITE_API_URL in `.env`
3. Verify CORS is enabled on API

### Editor Not Loading

**Problem:** Monaco Editor shows blank

**Solution:**
1. Clear browser cache
2. Check browser console for errors
3. Verify `@monaco-editor/react` is installed

### Build Errors

**Problem:** TypeScript errors during build

**Solution:**
```bash
# Clear node_modules and reinstall
rm -rf node_modules package-lock.json
npm install

# Run type check
npm run build
```

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

Monaco Editor requires modern browser with ES6+ support.

## Performance

- Initial load: ~2-3s
- Template load: ~100-200ms
- Validation: ~50-100ms (debounced)
- Compilation: ~100-200ms
- SDK generation: ~200-500ms

## Accessibility

- Semantic HTML
- ARIA labels on interactive elements
- Keyboard navigation support
- Screen reader friendly status messages

## Future Enhancements

- [ ] Form-based DSL editor (alternative to JSON)
- [ ] Template search and filtering
- [ ] DSL history and versioning
- [ ] Collaborative editing
- [ ] Dark mode
- [ ] Mobile responsive layout
- [ ] Authentication integration

## Contributing

1. Create feature branch
2. Make changes
3. Run `npm run build` to check for errors
4. Submit PR

## License

MIT

## Related

- [Logic Compiler API](../crates/logic-compiler-api/README.md)
- [Logic Compiler Library](../crates/logic-compiler/README.md)
- [Khafi Gateway Docs](../docs/)
