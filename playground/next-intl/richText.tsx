/**
 * next-intl のリッチテキスト機能
 *
 * t.rich() - リッチテキスト (コンポーネント埋め込み)
 * t.markup() - マークアップ (HTML タグ)
 * t.raw() - 生のテキスト
 */
import { useTranslations } from "next-intl";

// t.rich() の基本的な使い方
function RichTextBasic() {
  const t = useTranslations("rich");

  return (
    <div>
      {/* 実際のキー: rich.terms */}
      {t.rich("terms", {
        terms: (chunks) => <a href="/terms">{chunks}</a>,
      })}
    </div>
  );
}

// t.rich() でハイライト
function RichTextHighlight() {
  const t = useTranslations("rich");

  return (
    <div>
      {/* 実際のキー: rich.highlight */}
      {t.rich("highlight", {
        highlight: (chunks) => <mark>{chunks}</mark>,
      })}
    </div>
  );
}

// t.rich() でリンク
function RichTextLink() {
  const t = useTranslations("rich");

  return (
    <div>
      {/* 実際のキー: rich.link */}
      {t.rich("link", {
        link: (chunks) => <a href="/docs">{chunks}</a>,
      })}
    </div>
  );
}

// t.markup() の使い方
function MarkupText() {
  const t = useTranslations("markup");

  return (
    <div>
      {/* 実際のキー: markup.bold, markup.italic, markup.code */}
      <p>{t.markup("bold", { b: (chunks) => <strong>{chunks}</strong> })}</p>
      <p>{t.markup("italic", { i: (chunks) => <em>{chunks}</em> })}</p>
      <p>{t.markup("code", { code: (chunks) => <code>{chunks}</code> })}</p>
    </div>
  );
}

// t.raw() の使い方
function RawText() {
  const t = useTranslations("raw");

  return (
    <div>
      {/* 実際のキー: raw.html */}
      <div dangerouslySetInnerHTML={{ __html: t.raw("html") }} />

      {/* 実際のキー: raw.markdown */}
      <pre>{t.raw("markdown")}</pre>
    </div>
  );
}

// 複合的な使用
function CombinedUsage() {
  const t = useTranslations();

  return (
    <div>
      {/* 通常の t() */}
      <h1>{t("common.hello")}</h1>

      {/* t.rich() */}
      {t.rich("rich.terms", {
        terms: (chunks) => <a href="/terms">{chunks}</a>,
      })}

      {/* t.markup() */}
      <p>
        {t.markup("markup.bold", { b: (chunks) => <strong>{chunks}</strong> })}
      </p>

      {/* t.raw() */}
      <pre>{t.raw("raw.markdown")}</pre>
    </div>
  );
}

// namespace と組み合わせ
function WithNamespace() {
  const t = useTranslations("rich");

  return (
    <div>
      {/* すべて rich namespace 内 */}
      {t.rich("terms", {
        terms: (chunks) => <a href="/terms">{chunks}</a>,
      })}
      {t.rich("highlight", {
        highlight: (chunks) => <mark>{chunks}</mark>,
      })}
      {t.rich("link", {
        link: (chunks) => <a href="/docs">{chunks}</a>,
      })}
    </div>
  );
}

export {
  RichTextBasic,
  RichTextHighlight,
  RichTextLink,
  MarkupText,
  RawText,
  CombinedUsage,
  WithNamespace,
};
