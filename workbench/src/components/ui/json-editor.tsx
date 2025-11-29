"use client";

import { useRef, useEffect } from "react";
import { useTheme } from "next-themes";
import Editor, { Monaco } from "@monaco-editor/react";

interface JsonEditorProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  height?: string;
  readOnly?: boolean;
  className?: string;
  jsonSchema?: unknown; // JSON Schema for autocomplete
}

export function JsonEditor({
  value,
  onChange,
  height = "200px",
  readOnly = false,
  className,
  jsonSchema,
}: JsonEditorProps) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const editorRef = useRef<any>(null);
  const monacoRef = useRef<Monaco | null>(null);
  const modelUriRef = useRef<string | null>(null);
  const { theme, resolvedTheme } = useTheme();
  const monacoTheme = resolvedTheme === "dark" || theme === "dark" ? "vs-dark" : "vs";

  // Update schema when jsonSchema changes
  useEffect(() => {
    if (monacoRef.current && editorRef.current && modelUriRef.current) {
      const monaco = monacoRef.current;
      const modelUri = modelUriRef.current;
      
      // Update JSON schema configuration
      const schemas = jsonSchema
        ? [
            {
              uri: `http://localhost/schema-${Date.now()}.json`,
              fileMatch: [modelUri], // Match this specific model
              schema: jsonSchema,
            },
          ]
        : [];

      monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
        validate: true,
        allowComments: false,
        enableSchemaRequest: false,
        schemas,
        trailingCommas: "error",
        comments: "error",
      });
    }
  }, [jsonSchema]);

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const handleEditorDidMount = (editor: any, monaco: Monaco) => {
    editorRef.current = editor;
    monacoRef.current = monaco;

    const model = editor.getModel();
    if (model && model.getLanguageId() === "json") {
      const modelUri = model.uri.toString();
      modelUriRef.current = modelUri;

      const schemas = jsonSchema
        ? [
            {
              uri: `http://localhost/schema-${Date.now()}.json`,
              fileMatch: [modelUri],
              schema: jsonSchema,
            },
          ]
        : [];

      monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
        validate: true,
        allowComments: false,
        enableSchemaRequest: false,
        schemas,
        trailingCommas: "error",
        comments: "error",
      });
    }

    // Configure editor options for better autocomplete
    editor.updateOptions({
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      fontSize: 13,
      lineNumbers: "on",
      roundedSelection: false,
      cursorStyle: "line",
      automaticLayout: true,
      tabSize: 2,
      wordWrap: "on",
      formatOnPaste: true,
      formatOnType: true,
      suggestOnTriggerCharacters: true,
      quickSuggestions: {
        other: "on",
        comments: false,
        strings: "on",
      },
      suggestSelection: "first",
      tabCompletion: "on",
      suggest: {
        showKeywords: true,
        showSnippets: true,
        showClasses: true,
        showFunctions: true,
        showVariables: true,
        showFields: true,
        showConstructors: true,
        showEnums: true,
        showInterfaces: true,
        showModules: true,
        showProperties: true,
        showReferences: true,
        showTypeParameters: true,
        showUnits: true,
        showValues: true,
        showWords: true,
        showColors: true,
        showFiles: true,
        showFolders: true,
      },
      acceptSuggestionOnCommitCharacter: true,
      acceptSuggestionOnEnter: "on",
      snippetSuggestions: "top",
    });
  };

  const handleEditorChange = (value: string | undefined) => {
    onChange(value || "");
  };

  useEffect(() => {
    if (editorRef.current) {
    }
  }, [resolvedTheme, theme]);

  return (
    <div className={className}>
      <Editor
        height={height}
        defaultLanguage="json"
        value={value || ""}
        onChange={handleEditorChange}
        onMount={handleEditorDidMount}
        theme={monacoTheme}
        options={{
          readOnly,
          lineNumbers: "on",
          minimap: { enabled: false },
          scrollBeyondLastLine: false,
          fontSize: 13,
          wordWrap: "on",
          formatOnPaste: true,
          formatOnType: true,
          automaticLayout: true,
          tabSize: 2,
          suggestOnTriggerCharacters: true,
          quickSuggestions: {
            other: "on",
            comments: false,
            strings: "on",
          },
          acceptSuggestionOnCommitCharacter: true,
          acceptSuggestionOnEnter: "on",
          snippetSuggestions: "top",
          wordBasedSuggestions: "allDocuments",
          suggestSelection: "first",
          tabCompletion: "on",
        }}
        loading={
          <div className="flex items-center justify-center h-full bg-muted/50">
            <div className="text-sm text-muted-foreground">Loading editor...</div>
          </div>
        }
      />
    </div>
  );
}

