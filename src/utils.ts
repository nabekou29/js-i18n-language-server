import { resolve } from 'node:path';

import { readFileSync } from 'node:fs';

const queryCache = new Map<string, string>();

/**
 * Loads a query file from disk with caching.
 * @param query - Query file name
 * @returns Query file content
 * @example
 * loadQueryFileWithCache('i18next.scm');
 */
export function loadQueryFileWithCache(query: string): string {
  if (!queryCache.has(query)) {
    queryCache.set(query, readFileSync(resolve(__dirname, `./queries/${query}`), 'utf-8'));
  }

  const queryStr = queryCache.get(query);
  if (!queryStr) {
    throw new Error(`Failed to load query file: ${query}`);
  }

  return queryStr;
}
