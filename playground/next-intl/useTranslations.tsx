/**
 * next-intl useTranslations の様々なパターン
 *
 * useTranslations(namespace?)
 * - namespace: 名前空間 (= キープレフィックスとして機能)
 */
import { useTranslations } from "next-intl";

// 基本的な使い方 (namespace なし)
function BasicUsage() {
  const t = useTranslations();

  return (
    <div>
      <h1>{t("common.hello")}</h1>
      <p>{t("common.goodbye")}</p>
    </div>
  );
}

// namespace を指定 (= keyPrefix として機能)
function WithNamespace() {
  const t = useTranslations("common");

  // 実際のキー: common.hello, common.goodbye
  return (
    <div>
      <h1>{t("hello")}</h1>
      <p>{t("goodbye")}</p>
    </div>
  );
}

// 変数を含む翻訳
function WithInterpolation() {
  const t = useTranslations("common");

  // 実際のキー: common.welcome
  return <h1>{t("welcome", { name: "John" })}</h1>;
}

// ネストした namespace
function NestedNamespace() {
  const t = useTranslations("home.hero");

  // 実際のキー: home.hero.heading, home.hero.subheading
  return (
    <section>
      <h1>{t("heading")}</h1>
      <p>{t("subheading")}</p>
    </section>
  );
}

// ナビゲーションコンポーネント
function Navigation() {
  const t = useTranslations("navigation");

  // 実際のキー: navigation.home, navigation.about, navigation.contact
  return (
    <nav>
      <a href="/">{t("home")}</a>
      <a href="/about">{t("about")}</a>
      <a href="/contact">{t("contact")}</a>
      <a href="/products">{t("products")}</a>
    </nav>
  );
}

// 複数の useTranslations
function MultipleHooks() {
  const tCommon = useTranslations("common");
  const tNav = useTranslations("navigation");
  const tHome = useTranslations("home");

  return (
    <div>
      <header>
        <h1>{tCommon("hello")}</h1>
        <nav>
          <a href="/">{tNav("home")}</a>
          <a href="/about">{tNav("about")}</a>
        </nav>
      </header>
      <main>
        <h2>{tHome("title")}</h2>
        <p>{tHome("description")}</p>
      </main>
    </div>
  );
}

// ネストしたコンポーネント
function ParentComponent() {
  const tCommon = useTranslations("common");

  function ChildComponent() {
    const tNav = useTranslations("navigation");

    return (
      <nav>
        <a href="/">{tNav("home")}</a>
      </nav>
    );
  }

  return (
    <div>
      <h1>{tCommon("hello")}</h1>
      <ChildComponent />
    </div>
  );
}

// 条件分岐での使用
function ConditionalUsage() {
  const t = useTranslations("common");
  const isLoggedIn = true;

  return (
    <div>{isLoggedIn ? t("welcome", { name: "User" }) : t("hello")}</div>
  );
}

export {
  BasicUsage,
  WithNamespace,
  WithInterpolation,
  NestedNamespace,
  Navigation,
  MultipleHooks,
  ParentComponent,
  ConditionalUsage,
};
