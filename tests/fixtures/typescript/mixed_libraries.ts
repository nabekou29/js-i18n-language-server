// 複数のi18nライブラリが混在するケース
import i18n from 'i18next';
import { useTranslations, useLocale } from 'next-intl';
import { useTranslation } from 'react-i18next';
import { createI18n } from 'vue-i18n';

// i18next使用
function i18nextUsage() {
    const message = i18n.t('i18next.message');
    const greeting = i18n.t('i18next.greeting', { name: 'World' });
    
    return { message, greeting };
}

// react-i18next使用
function reactI18nextUsage() {
    const { t } = useTranslation('react-i18next');
    
    const title = t('title');
    const description = t('description');
    
    return { title, description };
}

// next-intl使用
function nextIntlUsage() {
    const t = useTranslations('next-intl');
    const locale = useLocale();
    
    const welcome = t('welcome');
    const currentLocale = t('current_locale', { locale });
    
    return { welcome, currentLocale };
}

// 混在使用（実際にはアンチパターンだが検出すべき）
function mixedUsage() {
    // 複数ライブラリの同時使用
    const { t: reactT } = useTranslation('react');
    const nextT = useTranslations('next');
    
    const reactMessage = reactT('mixed.react');
    const nextMessage = nextT('mixed.next');
    const i18nextMessage = i18n.t('mixed.i18next');
    
    return {
        react: reactMessage,
        next: nextMessage,
        i18next: i18nextMessage
    };
}

// ライブラリ特有の機能
function librarySpecificFeatures() {
    // react-i18nextの高度な機能
    const { t, i18n: i18nInstance } = useTranslation();
    
    // ネームスペース付き
    const adminT = useTranslation('admin').t;
    
    // keyPrefix付き
    const userT = useTranslation('user', { keyPrefix: 'profile' }).t;
    
    // next-intlの機能
    const intlT = useTranslations('intl');
    const locale = useLocale();
    
    return {
        // react-i18next
        basic: t('basic'),
        admin: adminT('panel'),
        userProfile: userT('name'), // user:profile.name
        
        // next-intl
        intlMessage: intlT('message'),
        localeInfo: locale,
        
        // i18next直接
        direct: i18n.t('direct.access')
    };
}

// 動的ライブラリ選択
function dynamicLibrarySelection(libraryType: 'react' | 'next' | 'i18next') {
    let getMessage: (key: string) => string;
    
    switch (libraryType) {
        case 'react':
            const { t: reactT } = useTranslation('dynamic');
            getMessage = reactT;
            break;
        case 'next':
            const nextT = useTranslations('dynamic');
            getMessage = nextT;
            break;
        case 'i18next':
        default:
            getMessage = (key: string) => i18n.t(`dynamic.${key}`);
    }
    
    return {
        welcome: getMessage('welcome'),
        description: getMessage('description')
    };
}

// カスタムフック内での複数ライブラリ使用
function useMultiLibraryTranslation() {
    const { t: reactT } = useTranslation('multi');
    const nextT = useTranslations('multi');
    
    return {
        fromReact: (key: string) => reactT(key),
        fromNext: (key: string) => nextT(key),
        fromI18next: (key: string) => i18n.t(`multi.${key}`)
    };
}

// Vue.js i18n (参考用、実際のJSでは使用しない)
// const vueI18n = createI18n({
//     locale: 'en',
//     messages: {
//         en: {
//             vue: {
//                 message: 'Hello from Vue i18n'
//             }
//         }
//     }
// });

// ライブラリ判定が必要なケース
interface I18nConfig {
    library: 'react-i18next' | 'next-intl' | 'i18next';
    namespace?: string;
    keyPrefix?: string;
}

function configBasedTranslation(config: I18nConfig, key: string): string {
    switch (config.library) {
        case 'react-i18next':
            const { t } = useTranslation(config.namespace, { 
                keyPrefix: config.keyPrefix 
            });
            return t(key);
            
        case 'next-intl':
            const nextT = useTranslations(config.namespace || 'default');
            return nextT(key);
            
        case 'i18next':
        default:
            const fullKey = config.namespace 
                ? `${config.namespace}:${config.keyPrefix ? config.keyPrefix + '.' : ''}${key}`
                : key;
            return i18n.t(fullKey);
    }
}

// エラーケース：間違ったライブラリ使用
function incorrectLibraryUsage() {
    // react-i18nextでnext-intlの書き方
    const wrongT = useTranslation('wrong'); // next-intlのuseTranslationsと混同
    
    // next-intlでreact-i18nextの書き方
    const { t: alsoWrong } = useTranslations('also-wrong'); // destructuringは不正
    
    return {
        wrong: wrongT.t('message'), // .tは不要
        alsoWrong: alsoWrong('message') // 関数が存在しない
    };
}

export {
    i18nextUsage,
    reactI18nextUsage,
    nextIntlUsage,
    mixedUsage,
    librarySpecificFeatures,
    dynamicLibrarySelection,
    useMultiLibraryTranslation,
    configBasedTranslation,
    incorrectLibraryUsage
};