import { useRef, useCallback, useEffect } from 'react'
import Editor, { useMonaco, type OnMount, type OnChange } from '@monaco-editor/react'
import type * as MonacoType from 'monaco-editor'

// Known extension names for auto-completion
const KNOWN_EXTENSIONS = [
  'vscode',
  'cursor',
  'git',
  'docker',
  'python',
  'node',
  'rust',
  'go',
  'java',
  'ruby',
  'php',
  'dotnet',
  'aws-cli',
  'gcloud',
  'azure-cli',
  'terraform',
  'ansible',
  'kubectl',
  'helm',
  'postgres',
  'mysql',
  'redis',
  'mongodb',
  'nginx',
  'caddy',
  'zsh',
  'fish',
  'tmux',
  'neovim',
  'vim',
  'emacs',
  'htop',
  'ripgrep',
  'fd',
  'fzf',
  'bat',
  'eza',
  'jq',
  'yq',
  'httpie',
  'gh',
  'lazygit',
]

const PROVIDERS = ['fly', 'kubernetes', 'docker-compose', 'docker', 'devpod', 'e2b', 'runpod', 'northflank'] as const

const EXTENSION_PROFILES = [
  'minimal',
  'fullstack',
  'anthropic-dev',
  'systems',
  'enterprise',
  'devops',
  'mobile',
  'visionflow-core',
  'visionflow-data-scientist',
  'visionflow-creative',
  'visionflow-full',
] as const

export interface YamlEditorProps {
  value: string
  onChange?: (value: string) => void
  readOnly?: boolean
  height?: string | number
  className?: string
}

export function YamlEditor({ value, onChange, readOnly = false, height = '100%', className }: YamlEditorProps) {
  const monaco = useMonaco()
  const editorRef = useRef<MonacoType.editor.IStandaloneCodeEditor | null>(null)
  const disposablesRef = useRef<MonacoType.IDisposable[]>([])

  useEffect(() => {
    if (!monaco) return

    const completionProvider = monaco.languages.registerCompletionItemProvider('yaml', {
      triggerCharacters: [' ', '-', '\n', ':'],
      provideCompletionItems(model, position) {
        const word = model.getWordUntilPosition(position)
        const range: MonacoType.IRange = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        }

        const lineContent = model.getLineContent(position.lineNumber)
        const prevLineContent = position.lineNumber > 1 ? model.getLineContent(position.lineNumber - 1) : ''

        const suggestions: MonacoType.languages.CompletionItem[] = []

        // Extension name completions when in extensions.active or extensions.additional array context
        const prevTrimmed = prevLineContent.trim()
        const isInExtensionsList =
          prevTrimmed === 'active:' ||
          prevTrimmed === 'additional:' ||
          (lineContent.trim().startsWith('-') && isInExtensionsBlock(model, position))

        if (isInExtensionsList) {
          KNOWN_EXTENSIONS.forEach((ext) => {
            suggestions.push({
              label: ext,
              kind: monaco.languages.CompletionItemKind.Value,
              documentation: `Sindri extension: ${ext}`,
              insertText: ext,
              range,
            })
          })
          return { suggestions }
        }

        // Top-level key completions (no indentation)
        const lineIndent = lineContent.match(/^(\s*)/)?.[1]?.length ?? 0
        if (lineIndent === 0) {
          const topLevelKeys: Array<{ key: string; snippet: string; doc: string }> = [
            { key: 'version', snippet: 'version: "1.0"', doc: 'Configuration schema version' },
            { key: 'name', snippet: 'name: my-instance', doc: 'Deployment name (lowercase, alphanumeric, hyphens)' },
            { key: 'deployment', snippet: 'deployment:\n  provider: docker', doc: 'Deployment configuration' },
            { key: 'extensions', snippet: 'extensions:\n  profile: minimal', doc: 'Extension configuration' },
            { key: 'secrets', snippet: 'secrets:\n  - name: MY_SECRET\n    source: env', doc: 'Secrets to inject' },
            { key: 'providers', snippet: 'providers:\n  fly:\n    region: sjc', doc: 'Provider-specific settings' },
          ]
          topLevelKeys.forEach(({ key, snippet, doc }) => {
            suggestions.push({
              label: key,
              kind: monaco.languages.CompletionItemKind.Property,
              documentation: doc,
              insertText: snippet,
              insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
              range,
            })
          })
        }

        // Provider value completions
        if (lineContent.match(/^\s+provider:\s*/)) {
          PROVIDERS.forEach((p) => {
            suggestions.push({
              label: p,
              kind: monaco.languages.CompletionItemKind.EnumMember,
              documentation: `Deploy to ${p}`,
              insertText: p,
              range,
            })
          })
        }

        // Profile value completions
        if (lineContent.match(/^\s+profile:\s*/)) {
          EXTENSION_PROFILES.forEach((p) => {
            suggestions.push({
              label: p,
              kind: monaco.languages.CompletionItemKind.EnumMember,
              documentation: `Extension profile: ${p}`,
              insertText: p,
              range,
            })
          })
        }

        return { suggestions }
      },
    })

    disposablesRef.current.push(completionProvider)

    return () => {
      disposablesRef.current.forEach((d) => d.dispose())
      disposablesRef.current = []
    }
  }, [monaco])

  const handleEditorMount: OnMount = useCallback(
    (editor) => {
      editorRef.current = editor
      editor.updateOptions({
        readOnly,
        minimap: { enabled: false },
        lineNumbers: 'on',
        wordWrap: 'on',
        scrollBeyondLastLine: false,
        fontSize: 13,
        tabSize: 2,
        insertSpaces: true,
        renderWhitespace: 'boundary',
        bracketPairColorization: { enabled: true },
        guides: { indentation: true },
        scrollbar: { vertical: 'auto', horizontal: 'auto' },
      })
    },
    [readOnly],
  )

  const handleChange: OnChange = useCallback(
    (val) => {
      onChange?.(val ?? '')
    },
    [onChange],
  )

  return (
    <div className={className} style={{ height, overflow: 'hidden' }}>
      <Editor
        height={height}
        defaultLanguage="yaml"
        value={value}
        onChange={handleChange}
        onMount={handleEditorMount}
        theme="vs-dark"
        options={{
          readOnly,
          minimap: { enabled: false },
          lineNumbers: 'on',
          wordWrap: 'on',
          scrollBeyondLastLine: false,
          fontSize: 13,
          tabSize: 2,
          insertSpaces: true,
          renderWhitespace: 'boundary',
          scrollbar: { vertical: 'auto', horizontal: 'auto' },
        }}
        loading={
          <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
            Loading editor...
          </div>
        }
      />
    </div>
  )
}

function isInExtensionsBlock(model: MonacoType.editor.ITextModel, position: MonacoType.Position): boolean {
  for (let line = position.lineNumber - 1; line >= 1; line--) {
    const content = model.getLineContent(line)
    const trimmed = content.trim()
    if (trimmed === 'active:' || trimmed === 'additional:') return true
    if (trimmed === 'extensions:') return false
    if (content.match(/^[a-z]/) && !content.startsWith(' ')) return false
  }
  return false
}
