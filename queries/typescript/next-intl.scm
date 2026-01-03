;; useTranslations 関数呼び出し
;; 引数: (namespace?)
(variable_declarator
  name: (identifier) @i18n.get_trans_fn_name
  value:
    (call_expression
      function: (identifier) @use_translations (#eq? @use_translations "useTranslations")
      arguments: (arguments) @i18n.get_trans_fn_args
    )
) @i18n.get_trans_fn
