import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

/**
 * Loads test fixture data from a specified file path.
 *
 * @param path - The relative path to the fixture file from the tests/fixtures directory
 * @returns The loaded fixture content as a string
 *
 * @example
 * const jsonContent = loadFixture('path/to/test-data.json');
 */
export function loadFixture(path: string) {
  return readFileSync(resolve(__dirname, `./fixtures/${path}`), 'utf-8');
}
