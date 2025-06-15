// React i18next Transコンポーネントのテスト
import React from 'react';
import { Trans } from 'react-i18next';

function TransComponents() {
    return (
        <div>
            {/* 基本的なTransコンポーネント */}
            <Trans i18nKey="basic.trans">
                Hello World
            </Trans>
            
            {/* 自己終了タグ */}
            <Trans i18nKey="self.closing" />
            
            {/* 複雑なTransコンポーネント */}
            <Trans 
                i18nKey="complex.trans"
                values={{ name: 'John', count: 5 }}
                components={{ 
                    1: <strong />, 
                    2: <em /> 
                }}
            >
                Hello <strong>{{name}}</strong>, you have <em>{{count}}</em> messages.
            </Trans>
            
            {/* ネストされたTransコンポーネント */}
            <div className="nested">
                <Trans i18nKey="nested.parent">
                    This is a parent with <Trans i18nKey="nested.child">child content</Trans>
                </Trans>
            </div>
            
            {/* 動的キー */}
            <Trans i18nKey={`dynamic.${userType}.message`}>
                Dynamic key content
            </Trans>
            
            {/* 条件付きTransコンポーネント */}
            {isLoggedIn ? (
                <Trans i18nKey="auth.logged_in">Welcome back!</Trans>
            ) : (
                <Trans i18nKey="auth.logged_out">Please log in</Trans>
            )}
            
            {/* 配列内のTransコンポーネント */}
            {items.map((item, index) => (
                <Trans key={index} i18nKey="list.item" values={{ item }}>
                    Item: {{item}}
                </Trans>
            ))}
            
            {/* 関数内のTransコンポーネント */}
            {renderCustomContent()}
        </div>
    );
}

function renderCustomContent() {
    return (
        <Trans i18nKey="function.rendered">
            This content is rendered by a function
        </Trans>
    );
}

function NamespacedTransComponent() {
    return (
        <div>
            {/* ネームスペース付きTransコンポーネント */}
            <Trans i18nKey="admin:panel.title">
                Admin Panel
            </Trans>
            
            <Trans i18nKey="user:profile.title">
                User Profile
            </Trans>
        </div>
    );
}

function TransWithHooks() {
    const { t } = useTranslation('common');
    
    return (
        <div>
            {/* フックとTransコンポーネントの混在 */}
            <h1>{t('page.title')}</h1>
            <Trans i18nKey="page.description">
                This page demonstrates mixed usage
            </Trans>
            
            {/* フック内のネームスペース + Transコンポーネント */}
            <Trans i18nKey="common:mixed.content">
                Mixed content example
            </Trans>
        </div>
    );
}

// 無効なTransコンポーネント（テスト用）
function InvalidTransComponents() {
    return (
        <div>
            {/* キーが空 */}
            <Trans i18nKey="">Empty key</Trans>
            
            {/* キーが未定義 */}
            <Trans i18nKey={undefined}>Undefined key</Trans>
            
            {/* キーが数値 */}
            <Trans i18nKey={123}>Numeric key</Trans>
            
            {/* i18nKey属性がない */}
            <Trans>No i18nKey attribute</Trans>
        </div>
    );
}

export {
    TransComponents,
    NamespacedTransComponent,
    TransWithHooks,
    InvalidTransComponents
};