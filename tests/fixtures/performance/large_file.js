// 大きなファイルのパフォーマンステスト用ファイル
import { useTranslation } from 'react-i18next';
import i18n from 'i18next';

// 大量の翻訳呼び出しを含むファイル
function LargeComponent() {
    const { t } = useTranslation('large');

    // 100個の翻訳呼び出し
    const messages = {
        msg001: t('message.001'),
        msg002: t('message.002'),
        msg003: t('message.003'),
        msg004: t('message.004'),
        msg005: t('message.005'),
        msg006: t('message.006'),
        msg007: t('message.007'),
        msg008: t('message.008'),
        msg009: t('message.009'),
        msg010: t('message.010'),
        msg011: t('message.011'),
        msg012: t('message.012'),
        msg013: t('message.013'),
        msg014: t('message.014'),
        msg015: t('message.015'),
        msg016: t('message.016'),
        msg017: t('message.017'),
        msg018: t('message.018'),
        msg019: t('message.019'),
        msg020: t('message.020'),
        msg021: t('message.021'),
        msg022: t('message.022'),
        msg023: t('message.023'),
        msg024: t('message.024'),
        msg025: t('message.025'),
        msg026: t('message.026'),
        msg027: t('message.027'),
        msg028: t('message.028'),
        msg029: t('message.029'),
        msg030: t('message.030'),
        msg031: t('message.031'),
        msg032: t('message.032'),
        msg033: t('message.033'),
        msg034: t('message.034'),
        msg035: t('message.035'),
        msg036: t('message.036'),
        msg037: t('message.037'),
        msg038: t('message.038'),
        msg039: t('message.039'),
        msg040: t('message.040'),
        msg041: t('message.041'),
        msg042: t('message.042'),
        msg043: t('message.043'),
        msg044: t('message.044'),
        msg045: t('message.045'),
        msg046: t('message.046'),
        msg047: t('message.047'),
        msg048: t('message.048'),
        msg049: t('message.049'),
        msg050: t('message.050'),
        msg051: t('message.051'),
        msg052: t('message.052'),
        msg053: t('message.053'),
        msg054: t('message.054'),
        msg055: t('message.055'),
        msg056: t('message.056'),
        msg057: t('message.057'),
        msg058: t('message.058'),
        msg059: t('message.059'),
        msg060: t('message.060'),
        msg061: t('message.061'),
        msg062: t('message.062'),
        msg063: t('message.063'),
        msg064: t('message.064'),
        msg065: t('message.065'),
        msg066: t('message.066'),
        msg067: t('message.067'),
        msg068: t('message.068'),
        msg069: t('message.069'),
        msg070: t('message.070'),
        msg071: t('message.071'),
        msg072: t('message.072'),
        msg073: t('message.073'),
        msg074: t('message.074'),
        msg075: t('message.075'),
        msg076: t('message.076'),
        msg077: t('message.077'),
        msg078: t('message.078'),
        msg079: t('message.079'),
        msg080: t('message.080'),
        msg081: t('message.081'),
        msg082: t('message.082'),
        msg083: t('message.083'),
        msg084: t('message.084'),
        msg085: t('message.085'),
        msg086: t('message.086'),
        msg087: t('message.087'),
        msg088: t('message.088'),
        msg089: t('message.089'),
        msg090: t('message.090'),
        msg091: t('message.091'),
        msg092: t('message.092'),
        msg093: t('message.093'),
        msg094: t('message.094'),
        msg095: t('message.095'),
        msg096: t('message.096'),
        msg097: t('message.097'),
        msg098: t('message.098'),
        msg099: t('message.099'),
        msg100: t('message.100')
    };

    return messages;
}

// 深いネストを持つコンポーネント
function DeeplyNestedComponent() {
    const { t } = useTranslation('nested');

    function level1() {
        const msg1 = t('level1.message');
        
        function level2() {
            const msg2 = t('level2.message');
            
            function level3() {
                const msg3 = t('level3.message');
                
                function level4() {
                    const msg4 = t('level4.message');
                    
                    function level5() {
                        const msg5 = t('level5.message');
                        return { msg5 };
                    }
                    
                    return { msg4, ...level5() };
                }
                
                return { msg3, ...level4() };
            }
            
            return { msg2, ...level3() };
        }
        
        return { msg1, ...level2() };
    }

    return level1();
}

// 複数のスコープを持つ大きなコンポーネント
function MultiScopeComponent() {
    const { t: commonT } = useTranslation('common');
    const { t: userT } = useTranslation('user');
    const { t: adminT } = useTranslation('admin');
    const { t: errorT } = useTranslation('errors');
    const { t: validationT } = useTranslation('validation');

    const commonMessages = [];
    const userMessages = [];
    const adminMessages = [];
    const errorMessages = [];
    const validationMessages = [];

    // 各スコープで50個ずつの翻訳呼び出し
    for (let i = 1; i <= 50; i++) {
        const num = i.toString().padStart(3, '0');
        commonMessages.push(commonT(`common.${num}`));
        userMessages.push(userT(`user.${num}`));
        adminMessages.push(adminT(`admin.${num}`));
        errorMessages.push(errorT(`error.${num}`));
        validationMessages.push(validationT(`validation.${num}`));
    }

    return {
        common: commonMessages,
        user: userMessages,
        admin: adminMessages,
        error: errorMessages,
        validation: validationMessages
    };
}

// 動的キーを大量に含むコンポーネント
function DynamicKeyComponent() {
    const { t } = useTranslation('dynamic');

    const categories = [
        'electronics', 'clothing', 'books', 'sports', 'home',
        'beauty', 'toys', 'automotive', 'health', 'garden'
    ];

    const subcategories = [
        'featured', 'new', 'bestsellers', 'discounted', 'premium'
    ];

    const actions = [
        'view', 'add', 'edit', 'delete', 'share', 'favorite'
    ];

    const results = {};

    // ネストされたループで動的キーを生成
    categories.forEach(category => {
        results[category] = {};
        
        subcategories.forEach(subcategory => {
            results[category][subcategory] = {};
            
            actions.forEach(action => {
                // 動的キーでの翻訳呼び出し
                const key = `${category}.${subcategory}.${action}`;
                results[category][subcategory][action] = t(key);
            });
        });
    });

    return results;
}

// メモリ使用量テスト用の大きなオブジェクト
function MemoryIntensiveComponent() {
    const { t } = useTranslation('memory');

    const largeData = [];

    // 1000個のオブジェクトを生成
    for (let i = 0; i < 1000; i++) {
        largeData.push({
            id: i,
            title: t(`item.${i}.title`),
            description: t(`item.${i}.description`),
            category: t(`category.${i % 10}.name`),
            status: t(`status.${i % 5}.label`),
            metadata: {
                created: t('metadata.created'),
                updated: t('metadata.updated'),
                author: t(`author.${i % 20}.name`)
            }
        });
    }

    return largeData;
}

// 複雑な条件分岐を含むコンポーネント
function ComplexConditionalComponent({ userRole, permissions, features }) {
    const { t } = useTranslation('conditional');

    const getMessages = () => {
        const messages = [];

        // 複雑な条件分岐
        if (userRole === 'admin') {
            messages.push(t('admin.welcome'));
            
            if (permissions.includes('user_management')) {
                messages.push(t('admin.user_management.title'));
                
                if (features.includes('bulk_operations')) {
                    messages.push(t('admin.user_management.bulk_operations'));
                }
                
                if (features.includes('advanced_search')) {
                    messages.push(t('admin.user_management.advanced_search'));
                }
            }
            
            if (permissions.includes('system_settings')) {
                messages.push(t('admin.system_settings.title'));
                
                for (let i = 0; i < 10; i++) {
                    messages.push(t(`admin.system_settings.option_${i}`));
                }
            }
            
        } else if (userRole === 'moderator') {
            messages.push(t('moderator.welcome'));
            
            permissions.forEach(permission => {
                messages.push(t(`moderator.${permission}.description`));
            });
            
        } else if (userRole === 'user') {
            messages.push(t('user.welcome'));
            
            features.forEach(feature => {
                if (permissions.includes(feature)) {
                    messages.push(t(`user.${feature}.enabled`));
                } else {
                    messages.push(t(`user.${feature}.disabled`));
                }
            });
            
        } else {
            messages.push(t('guest.welcome'));
            
            for (let i = 0; i < 5; i++) {
                messages.push(t(`guest.feature_${i}.description`));
            }
        }

        return messages;
    };

    return getMessages();
}

export {
    LargeComponent,
    DeeplyNestedComponent,
    MultiScopeComponent,
    DynamicKeyComponent,
    MemoryIntensiveComponent,
    ComplexConditionalComponent
};