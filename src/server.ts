import {
  type CompletionItem,
  CompletionItemKind,
  Diagnostic,
  DiagnosticSeverity,
  DidChangeConfigurationNotification,
  InitializeParams,
  InitializeResult,
  ProposedFeatures,
  type TextDocumentPositionParams,
  TextDocumentSyncKind,
  TextDocuments,
  createConnection,
} from 'vscode-languageserver/lib/node/main.js';

export function startServer() {
  const connection = createConnection(ProposedFeatures.all);

  connection.onInitialize((params) => {
    return {
      capabilities: {
        // textDocumentSync: TextDocumentSyncKind.Incremental,
        // definitionProvider: true,
        // referencesProvider: true,
        // hoverProvider: true,
        // completionProvider: {},
        // codeActionProvider: {},
        // executeCommandProvider: {
        //   commands: ['i18n.editTranslation'],
        // },
      },
    };
  });

  connection.onInitialized(() => {
    connection.console.log('[js-i18n] initialized');
  });

  connection.onCompletion((params: TextDocumentPositionParams): CompletionItem[] => {
    connection.console.log('[js-i18n] completion');

    return [];
  });

  connection.listen();
}
