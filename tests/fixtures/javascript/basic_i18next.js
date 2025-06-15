// 基本的なi18next使用パターン
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

// 基本的な翻訳呼び出し
const message = i18n.t('hello.world');
const greeting = i18n.t('greeting.message', { name: 'User' });

// 短縮形
const shortMessage = t('short.message');

// 動的キー（部分）
const dynamicKey = 'user';
const dynamicMessage = i18n.t(`${dynamicKey}.profile`);

// ネストした翻訳
const nestedTranslation = i18n.t('nested.deeply.buried.key');

// 条件付き翻訳
const condition = true;
const conditionalMessage = condition ? i18n.t('condition.true') : i18n.t('condition.false');

// 関数内での翻訳
function getMessage() {
    return i18n.t('function.message');
}

// オブジェクト内での翻訳
const messageObj = {
    title: i18n.t('object.title'),
    description: i18n.t('object.description')
};

// 配列内での翻訳
const messages = [
    i18n.t('array.first'),
    i18n.t('array.second'),
    i18n.t('array.third')
];

// 無効なキー（テスト用）
const invalidKey = i18n.t('');
const undefinedKey = i18n.t(undefined);

// コメント内のキー（無視されるべき）
// const commentKey = i18n.t('comment.key');
/* const blockCommentKey = i18n.t('block.comment.key'); */

// 文字列の中のキー（無視されるべき）
const stringWithKey = "This string contains i18n.t('string.key') but should be ignored";