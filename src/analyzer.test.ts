import { describe, expect, test } from 'vitest';

import { loadFixture } from '../tests/utils.js';
import { findTFunctionCalls } from './analyzer.js';
import type { Framework } from './framework/base.js';
import { BuiltInFrameworks } from './framework/index.js';

describe('findTFunctionCalls', () => {
  const i18next = new BuiltInFrameworks.I18Next();
  const nextIntl = new BuiltInFrameworks.NextIntl();

  function test_findTFunctionCalls(
    framework: Framework,
    file: string,
    language: string,
    expected: { key: string; keyPrefix: string; keyArg: string }[],
  ) {
    test(`should find t-function calls in ${file} (${framework.name})`, () => {
      // Arrange
      const testFile = loadFixture(file);

      // Act
      const result = findTFunctionCalls(testFile, framework, language).map((x) => ({
        key: x.key,
        keyPrefix: x.keyPrefix,
        keyArg: x.keyArg,
      }));

      // Assert
      expect(result).toHaveLength(expected.length);
      expect(result).toEqual(expected.map((x) => expect.objectContaining(x)));
    });
  }

  test_findTFunctionCalls(
    i18next,
    'analyzer/key-prefix/i18next/normal.js',
    'javascript',
    // biome-ignore format: expected table should not be formatted
    [
      { key: 'no-prefix-key-1',         keyPrefix: '',         keyArg: 'no-prefix-key-1' },
      { key: 'prefix-1.prefix-1-key-1', keyPrefix: 'prefix-1', keyArg: 'prefix-1-key-1' },
      { key: 'prefix-2.prefix-2-key-1', keyPrefix: 'prefix-2', keyArg: 'prefix-2-key-1' },
      { key: 'prefix-1.prefix-1-key-2', keyPrefix: 'prefix-1', keyArg: 'prefix-1-key-2' },
      { key: 'no-prefix-key-2',         keyPrefix: '',         keyArg: 'no-prefix-key-2' },
    ],
  );

  test_findTFunctionCalls(
    i18next,
    'analyzer/key-prefix/i18next/normal.jsx',
    'jsx',
    // biome-ignore format: expected table should not be formatted
    [
      { key: 'no-prefix-key-1',                 keyPrefix: '',             keyArg: 'no-prefix-key-1' },
      { key: 'prefix-1.prefix-1-key-1',         keyPrefix: 'prefix-1',     keyArg: 'prefix-1-key-1' },
      { key: 'no-prefix-key-2',                 keyPrefix: '',             keyArg: 'no-prefix-key-2' },
      { key: 'prefix-2.prefix-2-key-1',         keyPrefix: 'prefix-2',     keyArg: 'prefix-2-key-1' },
      { key: 'prefix-1.prefix-1-key-2',         keyPrefix: 'prefix-1',     keyArg: 'prefix-1-key-2' },
      { key: 'tsl-prefix-1.tsl-prefix-1-key-1', keyPrefix: 'tsl-prefix-1', keyArg: 'tsl-prefix-1-key-1' },
    ],
  );

  test_findTFunctionCalls(
    i18next,
    'analyzer/key-prefix/i18next/multiple-t-functions.jsx',
    'jsx',
    // biome-ignore format: expected table should not be formatted
    [
      { key: 't-prefix.key', keyPrefix: 't-prefix', keyArg: 'key' },
      { key: 't2-prefix.key', keyPrefix: 't2-prefix', keyArg: 'key' },
      { key: 't-prefix.key', keyPrefix: 't-prefix', keyArg: 'key' },
      { key: 't2-prefix.key', keyPrefix: 't2-prefix', keyArg: 'key' },
    ],
  );

  test_findTFunctionCalls(
    nextIntl,
    'analyzer/key-prefix/next-intl/normal.jsx',
    'jsx',
    // biome-ignore format: expected table should not be formatted
    [
      { key: 'no-prefix-key-1',         keyPrefix: '',         keyArg: 'no-prefix-key-1' },
      { key: 'prefix-1.prefix-1-key-1', keyPrefix: 'prefix-1', keyArg: 'prefix-1-key-1' },
      { key: 'no-prefix-key-2',         keyPrefix: '',         keyArg: 'no-prefix-key-2' },
      { key: 'prefix-2.prefix-2-key-1', keyPrefix: 'prefix-2', keyArg: 'prefix-2-key-1' },
      { key: 'prefix-1.prefix-1-key-2', keyPrefix: 'prefix-1', keyArg: 'prefix-1-key-2' },
    ],
  );

  test_findTFunctionCalls(
    nextIntl,
    'analyzer/key-prefix/next-intl/multiple-t-functions.jsx',
    'jsx',
    // biome-ignore format: expected table should not be formatted
    [
      { key: 't1-prefix.key', keyPrefix: 't1-prefix', keyArg: 'key' },
      { key: 't2-prefix.key', keyPrefix: 't2-prefix', keyArg: 'key' },
      { key: 't1-prefix.key', keyPrefix: 't1-prefix', keyArg: 'key' },
      { key: 't2-prefix.key', keyPrefix: 't2-prefix', keyArg: 'key' },
      { key: 't1-prefix.key', keyPrefix: 't1-prefix', keyArg: 'key' },
      { key: 't2-prefix.key', keyPrefix: 't2-prefix', keyArg: 'key' },
    ],
  );

  test.each`
    source
    ${`t('key')`}
    ${`t('key', { count: 1 })`}
    ${`i18next.t('key')`}
    ${`t(\n'key'\n)`}
    ${`<Trans i18nKey='key' t={t} />`}
    ${`<Trans i18nKey={'key'} t={t} />`}
    ${`<Trans i18nKey={'key'} t={t}></Trans>`}
  `('should find t-function calls in $source (i18next)', ({ source }) => {
    // Act
    const result = findTFunctionCalls(source, i18next, 'jsx');
    // Assert
    expect(result).toHaveLength(1);
  });

  test.each`
    source
    ${'t(variable)'}
    ${`tt('key')`}
    ${`<Trans i18nKey='does.not.hove.t-attr' />`}
    ${'<Trans i18nKey={variable} t={t} />'}
  `('should NOT find t-function calls in $source (i18next)', ({ source }) => {
    // Act
    const result = findTFunctionCalls(source, i18next, 'jsx');
    // Assert
    expect(result).toHaveLength(0);
  });

  test.each`
    source
    ${`t('key')`}
    ${`t('key', { count: 1 })`}
    ${`t(\n'key'\n)`}
    ${`t.rich('key', { guidelines: (chunks) => <a href="/guidelines">{chunks}</a> })`}
    ${`t.markup('markup', { important: (chunks) => \`<b>\${chunks}</b>\` })`}
    ${`t.raw('key')`}
  `('should find t-function calls in $source (next-intl)', ({ source }) => {
    // Act
    const result = findTFunctionCalls(source, nextIntl, 'jsx');
    // Assert
    expect(result).toHaveLength(1);
  });

  test.each`
    source
    ${'t(variable)'}
    ${`t('ke' + 'y')`}
    ${'t(`key`)'}
    ${"t.hoge('key')"}
  `('should NOT find t-function calls in $source (next-intl)', ({ source }) => {
    // Act
    const result = findTFunctionCalls(source, nextIntl, 'jsx');
    // Assert
    expect(result).toHaveLength(0);
  });
});
