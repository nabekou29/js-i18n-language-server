// 動的キーとエッジケースのテスト
import { useTranslation } from 'react-i18next';
import i18n from 'i18next';

function dynamicKeyPatterns() {
    const { t } = useTranslation('dynamic');
    
    // テンプレートリテラル
    const userType = 'admin';
    const dynamicMessage = t(`user.${userType}.welcome`);
    
    // 変数の組み合わせ
    const section = 'profile';
    const field = 'name';
    const fieldLabel = t(`${section}.${field}.label`);
    
    // 条件付き動的キー
    const status = 'active';
    const statusMessage = status === 'active' 
        ? t(`status.${status}.message`)
        : t('status.inactive.message');
    
    // 関数からの動的キー
    const getErrorKey = (errorType) => `errors.${errorType}`;
    const errorMessage = t(getErrorKey('validation'));
    
    // 配列インデックスでの動的キー
    const steps = ['first', 'second', 'third'];
    const stepMessages = steps.map((step, index) => 
        t(`wizard.step.${index + 1}.${step}`)
    );
    
    // オブジェクトプロパティでの動的キー
    const config = { theme: 'dark', lang: 'en' };
    const themeMessage = t(`theme.${config.theme}.description`);
    
    // 計算されたキー
    const now = new Date();
    const timeKey = now.getHours() < 12 ? 'morning' : 'evening';
    const greeting = t(`greetings.${timeKey}`);
    
    return {
        dynamicMessage,
        fieldLabel,
        statusMessage,
        errorMessage,
        stepMessages,
        themeMessage,
        greeting
    };
}

// 複雑な動的キー生成
function complexDynamicKeys() {
    const { t } = useTranslation('complex');
    
    // ネストされた関数での動的キー
    const createKeyBuilder = (namespace) => {
        return (section, item) => `${namespace}.${section}.${item}`;
    };
    
    const keyBuilder = createKeyBuilder('user');
    const profileKey = keyBuilder('profile', 'avatar');
    const message = t(profileKey);
    
    // 高次関数での動的キー
    const withPrefix = (prefix) => (key) => t(`${prefix}.${key}`);
    const adminT = withPrefix('admin');
    const userT = withPrefix('user');
    
    const adminMessage = adminT('dashboard.title');
    const userMessage = userT('profile.settings');
    
    // 再帰的な動的キー
    const buildNestedKey = (parts) => {
        if (parts.length === 0) return '';
        if (parts.length === 1) return parts[0];
        return parts.join('.');
    };
    
    const nestedKey = buildNestedKey(['deeply', 'nested', 'key', 'structure']);
    const nestedMessage = t(nestedKey);
    
    return {
        message,
        adminMessage,
        userMessage,
        nestedMessage
    };
}

// エラーケース：問題のある動的キー
function problematicDynamicKeys() {
    const { t } = useTranslation('problematic');
    
    // 未定義変数での動的キー
    const undefinedVar = undefined;
    const undefinedKey = t(`user.${undefinedVar}.message`); // "user.undefined.message"
    
    // null変数での動的キー
    const nullVar = null;
    const nullKey = t(`user.${nullVar}.message`); // "user.null.message"
    
    // 空文字列での動的キー
    const emptyVar = '';
    const emptyKey = t(`user.${emptyVar}.message`); // "user..message"
    
    // 数値での動的キー
    const numericVar = 42;
    const numericKey = t(`item.${numericVar}.name`); // "item.42.name"
    
    // 配列での動的キー（toString()が呼ばれる）
    const arrayVar = ['a', 'b', 'c'];
    const arrayKey = t(`list.${arrayVar}.title`); // "list.a,b,c.title"
    
    // オブジェクトでの動的キー（[object Object]になる）
    const objectVar = { type: 'user' };
    const objectKey = t(`type.${objectVar}.description`); // "type.[object Object].description"
    
    // 関数での動的キー
    const functionVar = () => 'function';
    const functionKey = t(`func.${functionVar}.result`); // 関数の文字列表現
    
    return {
        undefinedKey,
        nullKey,
        emptyKey,
        numericKey,
        arrayKey,
        objectKey,
        functionKey
    };
}

// コメント内のキー（検出されるべきでない）
function commentedKeys() {
    const { t } = useTranslation('comments');
    
    // const commented = t('commented.key');
    /* const blockCommented = t('block.commented.key'); */
    // TODO: t('todo.key') should be ignored
    /* 
     * Multi-line comment with t('multiline.key')
     * should also be ignored
     */
    
    const validKey = t('valid.key');
    
    return { validKey };
}

// 文字列内のキー（検出されるべきでない）
function stringEmbeddedKeys() {
    const { t } = useTranslation('strings');
    
    const documentation = "Use t('example.key') to translate this";
    const template = `
        To use i18n, call t('template.example') like this:
        const message = t('another.example');
    `;
    
    const jsCode = 'const message = t("code.example");';
    const htmlCode = '<Trans i18nKey="html.example">Example</Trans>';
    
    // 実際の翻訳呼び出し
    const realTranslation = t('real.translation');
    
    return {
        documentation,
        template,
        jsCode,
        htmlCode,
        realTranslation
    };
}

// 正規表現パターンでのキー
function regexPatternKeys() {
    const { t } = useTranslation('regex');
    
    const pattern = /t\('regex\.pattern\.key'\)/g;
    const regexString = "This regex matches t('regex.match.key')";
    
    // 実際の翻訳呼び出し
    const realKey = t('regex.real.key');
    
    return { pattern, regexString, realKey };
}

// 非同期処理での動的キー
async function asyncDynamicKeys() {
    const { t } = useTranslation('async');
    
    try {
        const data = await fetchUserData();
        const userTypeKey = t(`user.type.${data.type}`);
        
        return userTypeKey;
    } catch (error) {
        return t('async.error');
    }
}

// ジェネレータ関数での動的キー
function* generateTranslationKeys() {
    const { t } = useTranslation('generator');
    
    const keys = ['first', 'second', 'third'];
    
    for (const key of keys) {
        yield t(`generator.${key}.message`);
    }
}

export {
    dynamicKeyPatterns,
    complexDynamicKeys,
    problematicDynamicKeys,
    commentedKeys,
    stringEmbeddedKeys,
    regexPatternKeys,
    asyncDynamicKeys,
    generateTranslationKeys
};