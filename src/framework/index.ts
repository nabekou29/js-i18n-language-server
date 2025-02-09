import { BaseFramework, type Framework } from './base.js';

/**
 * i18next / react-i18next / next-i18next
 */
class I18Next extends BaseFramework implements Framework {
  name = 'i18next';

  globalTFunctionNames = ['t', 'i18next.t'];

  getQuery(language: string): string {
    return this.getQueryFromMap(
      {
        typescript: ['i18next.scm'],
        '*': ['i18next.scm', 'react-i18next.scm'],
      },
      language,
    );
  }
}

/**
 * next-intl
 */
class NextIntl extends BaseFramework implements Framework {
  name = 'next-intl';

  getQuery(language: string): string {
    return this.getQueryFromMap(
      {
        '*': ['next-intl.scm'],
      },
      language,
    );
  }
}

/** Built-in frameworks */
export const BuiltInFrameworks = {
  I18Next,
  NextIntl,
} as const;
