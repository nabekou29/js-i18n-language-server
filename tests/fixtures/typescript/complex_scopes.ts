// 複雑なスコープのテストケース
import { useTranslation } from 'react-i18next';
import i18n from 'i18next';

interface User {
    name: string;
    role: 'admin' | 'user';
}

class UserManager {
    private t: (key: string) => string;
    
    constructor() {
        // クラス内でのi18n初期化
        this.t = i18n.getFixedT('en', 'user-manager');
    }
    
    public getMessage(user: User): string {
        // クラスメソッド内での翻訳
        if (user.role === 'admin') {
            return this.t('admin.welcome');
        }
        return this.t('user.welcome');
    }
    
    public getNestedMessage(): string {
        // ネストされたメソッド内での翻訳
        const formatMessage = (key: string): string => {
            return this.t(`format.${key}`);
        };
        
        return formatMessage('nested');
    }
}

function complexScopeFunction() {
    const { t: commonT } = useTranslation('common');
    const { t: errorT } = useTranslation('errors');
    
    // 複数のスコープが混在する関数
    const processUser = async (userId: string) => {
        try {
            // 非同期処理内での翻訳
            const loading = commonT('loading.user');
            console.log(loading);
            
            const user = await fetchUser(userId);
            
            if (!user) {
                throw new Error(errorT('user.not_found'));
            }
            
            // ネストされたスコープでの翻訳
            const validateUser = (user: User) => {
                if (!user.name) {
                    return errorT('validation.name_required');
                }
                
                // さらにネストされたスコープ
                const checkRole = () => {
                    switch (user.role) {
                        case 'admin':
                            return commonT('role.admin');
                        case 'user':
                            return commonT('role.user');
                        default:
                            return errorT('role.invalid');
                    }
                };
                
                return checkRole();
            };
            
            return validateUser(user);
            
        } catch (error) {
            return errorT('generic.error');
        }
    };
    
    return processUser;
}

// ジェネリック関数での翻訳
function createTypedTranslator<T extends string>(namespace: T) {
    const { t } = useTranslation(namespace);
    
    return {
        translate: (key: string) => t(key),
        translateWithPrefix: (prefix: string, key: string) => t(`${prefix}.${key}`)
    };
}

// 高次関数での翻訳
function withTranslation<P extends object>(Component: React.ComponentType<P>) {
    return function WrappedComponent(props: P) {
        const { t } = useTranslation('hoc');
        
        // HOC内での翻訳
        const title = t('hoc.title');
        const description = t('hoc.description');
        
        return (
            <div>
                <h1>{title}</h1>
                <p>{description}</p>
                <Component {...props} />
            </div>
        );
    };
}

// 複雑な条件分岐での翻訳
function conditionalTranslations(userType: string, permissions: string[]) {
    let t: (key: string) => string;
    
    // 動的なフック選択（実際にはアンチパターン）
    switch (userType) {
        case 'admin':
            ({ t } = useTranslation('admin'));
            break;
        case 'moderator':
            ({ t } = useTranslation('moderator'));
            break;
        default:
            ({ t } = useTranslation('user'));
    }
    
    // 複雑な条件での翻訳キー生成
    const getPermissionMessage = () => {
        if (permissions.includes('read') && permissions.includes('write')) {
            return t('permissions.full_access');
        } else if (permissions.includes('read')) {
            return t('permissions.read_only');
        } else if (permissions.includes('write')) {
            return t('permissions.write_only');
        } else {
            return t('permissions.no_access');
        }
    };
    
    return {
        welcome: t('welcome'),
        permissions: getPermissionMessage(),
        // 配列での複数キー
        messages: [
            t('message.first'),
            t('message.second'),
            t('message.third')
        ],
        // オブジェクトでの複数キー
        labels: {
            save: t('button.save'),
            cancel: t('button.cancel'),
            delete: t('button.delete')
        }
    };
}

// 再帰的な翻訳処理
function recursiveTranslation(depth: number, namespace: string): string {
    const { t } = useTranslation(namespace);
    
    if (depth <= 0) {
        return t('recursion.base_case');
    }
    
    const current = t('recursion.current', { depth });
    const next = recursiveTranslation(depth - 1, `${namespace}.nested`);
    
    return `${current} ${next}`;
}

// プロミスチェーン内での翻訳
async function promiseChainTranslation() {
    const { t } = useTranslation('async');
    
    return Promise.resolve(t('start'))
        .then(message => {
            const next = t('middle');
            return `${message} -> ${next}`;
        })
        .then(message => {
            const final = t('end');
            return `${message} -> ${final}`;
        })
        .catch(() => {
            return t('error');
        });
}

// 型安全な翻訳関数
interface TranslationKeys {
    'user.name': never;
    'user.email': never;
    'admin.panel': never;
}

function typedTranslation<K extends keyof TranslationKeys>(key: K): string {
    const { t } = useTranslation('typed');
    return t(key);
}

// 使用例
const userName = typedTranslation('user.name');
const userEmail = typedTranslation('user.email');
const adminPanel = typedTranslation('admin.panel');