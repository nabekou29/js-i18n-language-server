// TypeScript + JSX の複雑なパターン
import React, { useState, useEffect, useCallback } from 'react';
import { useTranslation, Trans } from 'react-i18next';

interface User {
    id: number;
    name: string;
    role: 'admin' | 'user' | 'guest';
    preferences: {
        language: string;
        theme: 'light' | 'dark';
    };
}

interface ComponentProps {
    user: User;
    onUpdate?: (user: User) => void;
}

// TypeScriptの型安全性とJSXの組み合わせ
const MixedPatternComponent: React.FC<ComponentProps> = ({ user, onUpdate }) => {
    const { t } = useTranslation('mixed');
    const [isEditing, setIsEditing] = useState(false);
    const [localUser, setLocalUser] = useState<User>(user);

    // 型安全な翻訳キー
    type TranslationKey = 
        | 'user.profile.title'
        | 'user.profile.name'
        | 'user.profile.role'
        | 'actions.edit'
        | 'actions.save'
        | 'actions.cancel';

    const safeT = useCallback((key: TranslationKey) => t(key), [t]);

    // 複雑なuseEffect内での翻訳
    useEffect(() => {
        if (user.role === 'admin') {
            document.title = t('admin.dashboard.title');
        } else {
            document.title = t('user.dashboard.title');
        }
    }, [user.role, t]);

    // 動的な役割ベースの翻訳
    const getRoleSpecificMessage = useCallback((role: User['role']): string => {
        switch (role) {
            case 'admin':
                return t('role.admin.description');
            case 'user':
                return t('role.user.description');
            case 'guest':
                return t('role.guest.description');
            default:
                return t('role.unknown.description');
        }
    }, [t]);

    // 複雑なイベントハンドラ内での翻訳
    const handleSave = useCallback(async () => {
        try {
            if (!localUser.name.trim()) {
                throw new Error(t('validation.name.required'));
            }

            if (onUpdate) {
                await onUpdate(localUser);
            }

            setIsEditing(false);
            
            // 成功メッセージ
            const successMessage = t('actions.save.success', { 
                name: localUser.name 
            });
            
            console.log(successMessage);
            
        } catch (error) {
            const errorMessage = t('actions.save.error');
            console.error(errorMessage, error);
        }
    }, [localUser, onUpdate, t]);

    // 条件付きレンダリング内でのTransコンポーネント
    const renderUserInfo = () => {
        if (isEditing) {
            return (
                <div className="editing-mode">
                    <Trans i18nKey="user.editing.title" values={{ name: localUser.name }}>
                        Editing profile for <strong>{{name: localUser.name}}</strong>
                    </Trans>
                    
                    <input
                        type="text"
                        value={localUser.name}
                        onChange={(e) => setLocalUser({ ...localUser, name: e.target.value })}
                        placeholder={t('user.name.placeholder')}
                        aria-label={t('user.name.aria_label')}
                    />
                    
                    <select
                        value={localUser.role}
                        onChange={(e) => setLocalUser({ 
                            ...localUser, 
                            role: e.target.value as User['role'] 
                        })}
                        aria-label={t('user.role.aria_label')}
                    >
                        <option value="user">{t('role.user.label')}</option>
                        <option value="admin">{t('role.admin.label')}</option>
                        <option value="guest">{t('role.guest.label')}</option>
                    </select>
                </div>
            );
        }

        return (
            <div className="view-mode">
                <h2>{safeT('user.profile.title')}</h2>
                
                <Trans 
                    i18nKey="user.profile.info" 
                    values={{ 
                        name: localUser.name, 
                        role: t(`role.${localUser.role}.label`) 
                    }}
                >
                    User: <strong>{{name: localUser.name}}</strong> 
                    (Role: {{role: t(`role.${localUser.role}.label`)}})
                </Trans>
                
                <p>{getRoleSpecificMessage(localUser.role)}</p>
            </div>
        );
    };

    // ネストされたコンポーネント内での翻訳
    const PreferencesSection: React.FC = () => {
        const { t: prefT } = useTranslation('preferences');
        
        return (
            <section className="preferences">
                <h3>{prefT('title')}</h3>
                
                <div className="preference-item">
                    <label htmlFor="language">
                        {prefT('language.label')}
                    </label>
                    <select 
                        id="language"
                        value={localUser.preferences.language}
                        onChange={(e) => setLocalUser({
                            ...localUser,
                            preferences: {
                                ...localUser.preferences,
                                language: e.target.value
                            }
                        })}
                    >
                        <option value="en">{prefT('language.en')}</option>
                        <option value="ja">{prefT('language.ja')}</option>
                        <option value="fr">{prefT('language.fr')}</option>
                    </select>
                </div>
                
                <div className="preference-item">
                    <label htmlFor="theme">
                        {prefT('theme.label')}
                    </label>
                    <select 
                        id="theme"
                        value={localUser.preferences.theme}
                        onChange={(e) => setLocalUser({
                            ...localUser,
                            preferences: {
                                ...localUser.preferences,
                                theme: e.target.value as 'light' | 'dark'
                            }
                        })}
                    >
                        <option value="light">{prefT('theme.light')}</option>
                        <option value="dark">{prefT('theme.dark')}</option>
                    </select>
                </div>
            </section>
        );
    };

    // エラー境界内での翻訳
    const ErrorDisplay: React.FC<{ error: string }> = ({ error }) => {
        const { t: errorT } = useTranslation('errors');
        
        return (
            <div className="error-display">
                <Trans i18nKey="errors.general.title">
                    An error occurred
                </Trans>
                
                <p>{errorT('general.message')}</p>
                <code>{error}</code>
                
                <button onClick={() => window.location.reload()}>
                    {errorT('actions.reload')}
                </button>
            </div>
        );
    };

    // 複雑な条件付きレンダリング
    return (
        <div className="mixed-pattern-component">
            <header>
                <Trans i18nKey="component.header.title" values={{ version: '2.0' }}>
                    User Management System v{{version: '2.0'}}
                </Trans>
            </header>

            <main>
                {renderUserInfo()}
                
                <PreferencesSection />
                
                {/* 動的なTransコンポーネント */}
                {localUser.role === 'admin' && (
                    <Trans i18nKey="admin.special.message">
                        You have administrative privileges
                    </Trans>
                )}
                
                {/* 配列での繰り返しレンダリング */}
                <section className="permissions">
                    <h3>{t('permissions.title')}</h3>
                    {['read', 'write', 'delete'].map(permission => (
                        <div key={permission} className="permission-item">
                            <Trans 
                                i18nKey="permissions.item" 
                                values={{ permission }}
                            >
                                Permission: {{permission}}
                            </Trans>
                            <span className="status">
                                {t(`permissions.${permission}.status`)}
                            </span>
                        </div>
                    ))}
                </section>
            </main>

            <footer>
                <div className="actions">
                    {isEditing ? (
                        <>
                            <button onClick={handleSave}>
                                {safeT('actions.save')}
                            </button>
                            <button onClick={() => setIsEditing(false)}>
                                {safeT('actions.cancel')}
                            </button>
                        </>
                    ) : (
                        <button onClick={() => setIsEditing(true)}>
                            {safeT('actions.edit')}
                        </button>
                    )}
                </div>
                
                <Trans i18nKey="component.footer.copyright" values={{ year: new Date().getFullYear() }}>
                    © {{year: new Date().getFullYear()}} My Company
                </Trans>
            </footer>
        </div>
    );
};

// 高次コンポーネントでの翻訳
function withI18nLogging<P extends object>(Component: React.ComponentType<P>) {
    return function WrappedComponent(props: P) {
        const { t } = useTranslation('hoc');
        
        useEffect(() => {
            console.log(t('hoc.component.mounted'));
            
            return () => {
                console.log(t('hoc.component.unmounted'));
            };
        }, [t]);
        
        return <Component {...props} />;
    };
}

// カスタムフックでの翻訳
function useUserMessages(user: User) {
    const { t } = useTranslation('user-messages');
    
    return {
        welcome: t('welcome', { name: user.name }),
        goodbye: t('goodbye', { name: user.name }),
        roleMessage: t(`role.${user.role}.message`),
        
        // 配列での複数メッセージ
        notifications: [
            t('notifications.new_message'),
            t('notifications.update_available'),
            t('notifications.maintenance')
        ],
        
        // 動的メッセージ生成
        generateStatusMessage: (status: string) => t(`status.${status}.message`)
    };
}

export default MixedPatternComponent;
export { withI18nLogging, useUserMessages };
export type { User, ComponentProps };