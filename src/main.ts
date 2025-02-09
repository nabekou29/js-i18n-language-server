#!/usr/bin/env node

import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { startServer } from './server.js';

const args = process.argv;

// Show version
if (args.includes('-v') || args.includes('--version')) {
  // __dirname is `js-i18n-language-server/dist`
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const packageJson = JSON.parse(readFileSync(join(__dirname, '..', 'package.json'), 'utf8'));
  process.stdout.write(`${packageJson.version}\n`);
  process.exit(0);
}

startServer();
