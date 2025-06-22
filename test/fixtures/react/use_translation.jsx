import React from "react";
import { useTranslation } from "react-i18next";

export const SimpleComponent = () => {
  const { t } = useTranslation();
  
  return (
    <div>
      <h1>{t("page.title")}</h1>
      <p>{t("page.description")}</p>
    </div>
  );
};

export const WithNamespace = () => {
  const { t } = useTranslation("common");
  
  return <button>{t("button.submit")}</button>;
};

export const WithKeyPrefix = () => {
  const { t } = useTranslation("translation", { keyPrefix: "home" });
  
  return (
    <section>
      <h2>{t("title")}</h2>
      <p>{t("intro")}</p>
    </section>
  );
};