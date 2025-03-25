import Parser, { type Language } from 'tree-sitter';
import javascript from 'tree-sitter-javascript';
import typescript from 'tree-sitter-typescript';

import type { Framework } from './framework/base.js';

const JavaScript = javascript as Language;
const TypeScript = typescript.typescript as Language;
const TSX = typescript.tsx as Language;

const QueryKey = {
  GetT: 'i18n.get_t',
  CallT: 'i18n.call_t',
  TFunctionName: 'i18n.t_func_name',
  Key: 'i18n.key',
  KeyArg: 'i18n.key_arg',
  Namespace: 'i18n.namespace',
  KeyPrefix: 'i18n.key_prefix',
} as const;

/**
 * Information about the t-function
 *
 * For example:
 *
 * ```tsx
 * const Component = () => {
 *   const { t: myT } = useTranslation("translation", { keyPrefix: "t-prefix" });
 *   ...
 * }
 * ```
 *
 * In this case:
 * - tFunctionName: "myT"
 * - namespace: "translation"
 * - keyPrefix: "t-prefix"
 * - scopeNode: The scope node inside the Component function
 */
type TFunction = {
  /** The name of the t-function */
  tFunctionName: string;
  /** The namespace of the t-function */
  namespace: string | null;
  /** The key prefix of the t-function */
  keyPrefix: string;
  /** The arguments of the t-function */
  scopeNode: Parser.SyntaxNode;
};

/**
 * Information about the t-function call
 *
 * For example:
 * ```tsx
 * const message = t("translation:path.to.key");
 * ```
 *
 * In this case:
 * - tFunctionName: "t"
 * - key: "path.to.key"
 * - keyNode: Node(`path.to.key`)
 *   keyArgNode: Node(`"path.to.key"`)
 *   namespace: "translation"
 */
type CallTFunction = {
  /** The name of the t-function */
  tFunctionName: string;
  /** The key of the t-function */
  key: string;
  /** The node of the key */
  keyNode: Parser.SyntaxNode;
  /** The node of the key argument */
  keyArgNode: Parser.SyntaxNode;
  /** The namespace of the t-function */
  namespace: string | null;
};

/**
 * The result of finding t-function calls
 *
 * For example:
 * ```tsx
 * const Component = () => {
 *  const { t: myT } = useTranslation("translation", { keyPrefix: "t-prefix" });
 *
 *  const message = myT("path");
 *  }
 *  ```
 *
 *  In this case:
 *  - node: Node(`myT("path")`)
 *  - keyNode: Node(`path`)
 *  - keyArgNode: Node(`"path"`)
 *  - key: "t-prefix.path"
 *  - keyArg: "path"
 *  - namespace: "translation"
 */
type FindTFunctionCallsResult = {
  /** Call t-function node */
  node: Parser.SyntaxNode;
  /** Key node */
  keyNode: Parser.SyntaxNode;
  /** Key argument node */
  keyArgNode: Parser.SyntaxNode;
  /** Key */
  key: string;
  /** Key prefix */
  keyPrefix: string;
  /** Key argument */
  keyArg: string;
  /** Namespace */
  namespace: string | null;
};

/**
 * Find calls to the t-function
 * @param sourceCode - The source code to analyze
 * @param framework - Which framework is used
 * @param language - The programming language of the source code
 */
export function findTFunctionCalls(
  sourceCode: string,
  framework: Framework,
  language: string,
): FindTFunctionCallsResult[] {
  const parserLanguage = getParserLanguage(language);
  const parser = new Parser();
  parser.setLanguage(parserLanguage);

  const tree = parser.parse(sourceCode);

  const query = new Parser.Query(parserLanguage, framework.getQuery(language));
  const captures = query.captures(tree.rootNode);

  const scopeStack = new ScopeStack();
  const results: FindTFunctionCallsResult[] = [];

  const isTFunc = (tFuncName: string): boolean =>
    framework.globalTFunctionNames.includes(tFuncName) || (scopeStack.stacks[tFuncName]?.length ?? 0) > 0;

  for (const { name, node } of captures) {
    // Exit the scope if the node is outside the scope
    for (const tFuncName in scopeStack.stacks) {
      let currentScope = scopeStack.getCurrentScope(tFuncName);
      while (currentScope && !includesNode(currentScope.scopeNode, node)) {
        scopeStack.exit(tFuncName);
        currentScope = scopeStack.getCurrentScope(tFuncName);
      }
    }

    if (name === QueryKey.GetT) {
      const getTDetail = parseGetTFunction(node, query);
      if (getTDetail) {
        // If the same name variable is assigned in the same scope, exit the scope.
        if (getTDetail.scopeNode === scopeStack.getCurrentScope(getTDetail.tFunctionName)?.scopeNode) {
          scopeStack.exit(getTDetail.tFunctionName);
        }
        scopeStack.enter(getTDetail);
      }
    } else if (name === QueryKey.CallT) {
      const callTDetail = parseCallTFunction(node, query);
      if (!callTDetail || !isTFunc(callTDetail.tFunctionName)) {
        continue;
      }

      const scope = scopeStack.getCurrentScope(callTDetail.tFunctionName);

      let key = callTDetail.key;
      if (scope?.keyPrefix) {
        key = framework.joinKey([scope.keyPrefix, ...framework.splitKey(key)]);
      }

      results.push({
        node,
        keyNode: callTDetail.keyNode,
        keyArgNode: callTDetail.keyArgNode,
        key,
        keyPrefix: scope?.keyPrefix ?? '',
        keyArg: callTDetail.key,
        namespace: (callTDetail.namespace || scope?.namespace) ?? null,
      });
    }
  }

  return results;
}

/**
 * Get the t-function details from the node
 */
function parseGetTFunction(targetNode: Parser.SyntaxNode, query: Parser.Query): TFunction | null {
  let tFunctionName = null;
  let namespace = null;
  let keyPrefix = '';

  const captures = query.captures(targetNode);

  for (const { name, node } of captures) {
    if (name === QueryKey.TFunctionName) {
      tFunctionName = tFunctionName || node.text;
    } else if (name === QueryKey.Namespace) {
      namespace = namespace || node.text;
    } else if (name === QueryKey.KeyPrefix) {
      keyPrefix = keyPrefix || node.text;
    }
  }

  const scopeNode = targetNode.closest(['statement_block', 'jsx_element']);

  if (!tFunctionName || !scopeNode) {
    return null;
  }

  return { tFunctionName, namespace, keyPrefix, scopeNode };
}

/**
 * Get the call-t function details from the node
 */
function parseCallTFunction(targetNode: Parser.SyntaxNode, query: Parser.Query): CallTFunction | null {
  let tFunctionName = null;
  let key = '';
  let keyNode = null;
  let keyArgNode = null;
  let namespace = '';

  for (const { name, node } of query.captures(targetNode)) {
    if (name === QueryKey.TFunctionName) {
      tFunctionName = tFunctionName || node.text;
    } else if (name === QueryKey.Key) {
      key = key || node.text;
      keyNode = keyNode || node;
    } else if (name === QueryKey.KeyArg) {
      keyArgNode = keyArgNode || node;
    } else if (name === QueryKey.Namespace) {
      namespace = namespace || node.text;
    }
  }

  if (!tFunctionName || !key || !keyNode || !keyArgNode) {
    return null;
  }

  return { tFunctionName, key, keyNode, keyArgNode, namespace };
}

/**
 * A class to manage scopes for t-functions.
 */
class ScopeStack {
  /**
   * The stack of scopes
   *
   * The key is the name of the t-function, and the value is an array of t-function calls.
   */
  stacks: Record<string, TFunction[]> = {};

  static DEFAULT_T_FUNCTION_NAME = '__t__';

  /**
   * Push a new scope to the stack
   */
  enter(value: TFunction): void {
    const tFuncName = value.tFunctionName || ScopeStack.DEFAULT_T_FUNCTION_NAME;
    this.stacks[tFuncName] = this.stacks[tFuncName] ?? [];
    this.stacks[tFuncName].push(value);
  }

  /**
   * Pop the last scope from the stack
   */
  exit(tFunctionName: string): void {
    this.stacks[tFunctionName || ScopeStack.DEFAULT_T_FUNCTION_NAME]?.pop();
  }

  /**
   * Get the current scope
   */
  getCurrentScope(tFunctionName: string): TFunction | null {
    const stack = this.stacks[tFunctionName || ScopeStack.DEFAULT_T_FUNCTION_NAME];
    return stack?.[stack.length - 1] ?? null;
  }
}

function getParserLanguage(language: string): Language {
  switch (language) {
    case 'javascript':
      return JavaScript;
    case 'typescript':
      return TypeScript;
    default:
      return TSX;
  }
}

function includesNode(node: Parser.SyntaxNode, targetNode: Parser.SyntaxNode): boolean {
  return node.startIndex <= targetNode.startIndex && node.endIndex >= targetNode.endIndex;
}
