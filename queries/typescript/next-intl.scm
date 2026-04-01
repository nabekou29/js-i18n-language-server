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

;; getTranslations 関数呼び出し (Server Components)
;; 引数: (namespace?) または ({ namespace?: string, locale?: string })
(variable_declarator
  name: (identifier) @i18n.get_trans_fn_name
  value:
    (await_expression
      (call_expression
        function: (identifier) @use_translations (#eq? @use_translations "getTranslations")
        arguments: (arguments) @i18n.get_trans_fn_args
      )
    )
) @i18n.get_trans_fn

;; getTranslations オブジェクト引数の namespace 抽出
;; await getTranslations({ namespace: "common" })
;; next-intl の namespace は key prefix として機能する
(variable_declarator
  name: (identifier) @i18n.get_trans_fn_name
  value:
    (await_expression
      (call_expression
        function: (identifier) @use_translations (#eq? @use_translations "getTranslations")
        arguments:
          (arguments
            (object
              (pair
                key: (property_identifier) @_ns_key (#eq? @_ns_key "namespace")
                value: (string (string_fragment) @i18n.trans_key_prefix)
              )
            )
          )
      )
    )
) @i18n.get_trans_fn
