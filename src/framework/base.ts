import { loadQueryFileWithCache } from '../utils.js';

export interface Framework {
  /** The name of the framework */
  name: string;

  /** The global t-function names */
  globalTFunctionNames: string[];

  /**
   * Get tree-sitter query for parsing
   *
   * @param language - Target language to parse
   * @returns tree-sitter query
   */
  getQuery(language: string): string;

  /**
   * Split concatenated keys
   *
   * @example
   * splitKey('a.b.c') // => ['a', 'b', 'c']
   */
  splitKey(key: string): string[];

  /**
   * Join keys with separator
   *
   * @example
   * joinKey(['a', 'b', 'c']) // => 'a.b.c'
   */
  joinKey(keys: string[]): string;

  /**
   * Get namespace from key
   *
   * @example
   * getNamespaceFromKey('namespace:key') // => 'namespace'
   * getNamespaceFromKey('key') // => null
   */
  getNamespaceFromKey(key: string): string | null;
}

/**
 * A class to absorb differences in processing for each framework
 */
export abstract class BaseFramework implements Framework {
  keySeparator: string;
  namespaceSeparator: string;

  constructor({
    keySeparator = '.',
    namespaceSeparator = ':',
  }: { keySeparator?: string; namespaceSeparator?: string } = {}) {
    this.keySeparator = keySeparator;
    this.namespaceSeparator = namespaceSeparator;
  }

  abstract name: string;

  globalTFunctionNames: string[] = ['t'];

  abstract getQuery(language: string): string;

  protected getQueryFromMap(map: Record<string, string[]>, language: string): string {
    const queries = map[language] || map['*'];
    if (!queries) {
      throw new Error(`No queries found for language: ${language}`);
    }

    return queries.map((query) => loadQueryFileWithCache(query)).join('\n');
  }

  splitKey(key: string): string[] {
    return key.split(this.keySeparator);
  }

  joinKey(keys: string[]): string {
    return keys.join(this.keySeparator);
  }

  getNamespaceFromKey(key: string): string | null {
    const parts = key.split(this.namespaceSeparator);
    if (parts.length === 0) {
      return null;
    }
    return parts[0] ?? null;
  }
}
