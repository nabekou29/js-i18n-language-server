; Call function to get `t`
;--------------------------------
(variable_declarator
  name: (identifier) @i18n.t_func_name
  value:
    (call_expression
      function: (identifier) @use_translations (#eq? @use_translations "useTranslations")
      arguments: (arguments
        [
          (string (string_fragment) @i18n.key_prefix)
          (undefined)
        ]?
      )
    )
) @i18n.get_t

; Call `t` function
;--------------------------------
; t("key")
(call_expression
  function: (identifier) @i18n.t_func_name
  arguments: (arguments
    (string
      (string_fragment) @i18n.key
    ) @i18n.key_arg
  )
) @i18n.call_t

; t.rich('key')
(call_expression
  function: 
    (member_expression
      object: (identifier) @i18n.t_func_name
      property: (property_identifier) @t_func_member (#any-of? @t_func_member "raw" "rich" "markup")
    )
  arguments: (arguments
    (string
      (string_fragment) @i18n.key
    ) @i18n.key_arg
  )
) @i18n.call_t
