;; useTranslations hook
;; Args: (namespace?)
(variable_declarator
  name: (identifier) @i18n.get_trans_fn_name
  value:
    (call_expression
      function: (identifier) @use_translations (#eq? @use_translations "useTranslations")
      arguments: (arguments) @i18n.get_trans_fn_args
    )
) @i18n.get_trans_fn

;; getTranslations (Server Components)
;; Args: (namespace?) or ({ namespace?: string, locale?: string })
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

;; getTranslations with object argument: namespace extraction
;; await getTranslations({ namespace: "common" })
;; In next-intl, namespace acts as a key prefix
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
