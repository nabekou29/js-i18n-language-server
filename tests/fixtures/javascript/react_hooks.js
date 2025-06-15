// React Hooksパターンのテスト
import React from 'react';
import { useTranslation } from 'react-i18next';

function MyComponent() {
    // 基本的なuseTranslation
    const { t } = useTranslation();
    
    return (
        <div>
            <h1>{t('component.title')}</h1>
            <p>{t('component.description')}</p>
        </div>
    );
}

function NamespacedComponent() {
    // ネームスペース付きuseTranslation
    const { t } = useTranslation('admin');
    
    return (
        <div>
            <h1>{t('panel.title')}</h1>
            <button>{t('panel.button.save')}</button>
            <button>{t('panel.button.cancel')}</button>
        </div>
    );
}

function PrefixedComponent() {
    // keyPrefix付きuseTranslation
    const { t } = useTranslation('user', { keyPrefix: 'profile' });
    
    return (
        <div>
            <h2>{t('name')}</h2> {/* user:profile.name */}
            <p>{t('email')}</p>  {/* user:profile.email */}
            <span>{t('phone')}</span> {/* user:profile.phone */}
        </div>
    );
}

function MultipleHooksComponent() {
    // 複数のuseTranslationフック
    const { t: tCommon } = useTranslation('common');
    const { t: tErrors } = useTranslation('errors');
    
    return (
        <div>
            <h1>{tCommon('app.title')}</h1>
            <div className="error">
                {tErrors('validation.required')}
            </div>
        </div>
    );
}

function NestedScopeComponent() {
    const { t } = useTranslation('dashboard');
    
    const handleClick = () => {
        // ネストされたスコープでの翻訳
        const confirmation = t('confirm.delete');
        
        if (window.confirm(confirmation)) {
            // さらにネストされたスコープ
            const success = t('success.deleted');
            alert(success);
        }
    };
    
    return (
        <div>
            <h1>{t('title')}</h1>
            <button onClick={handleClick}>
                {t('button.delete')}
            </button>
        </div>
    );
}

function ConditionalHooksComponent({ userType }) {
    // 条件付きフック使用（アンチパターンだが現実的）
    const { t } = userType === 'admin' 
        ? useTranslation('admin') 
        : useTranslation('user');
    
    return (
        <div>
            <h1>{t('welcome')}</h1>
            <p>{t('description')}</p>
        </div>
    );
}

// カスタムフック内でのuseTranslation
function useCustomMessages() {
    const { t } = useTranslation('messages');
    
    return {
        success: t('success'),
        error: t('error'),
        warning: t('warning')
    };
}

function CustomHookUser() {
    const messages = useCustomMessages();
    
    return (
        <div>
            <div className="success">{messages.success}</div>
            <div className="error">{messages.error}</div>
            <div className="warning">{messages.warning}</div>
        </div>
    );
}

export {
    MyComponent,
    NamespacedComponent,
    PrefixedComponent,
    MultipleHooksComponent,
    NestedScopeComponent,
    ConditionalHooksComponent,
    CustomHookUser
};