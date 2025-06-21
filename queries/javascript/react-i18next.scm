;; useTranslation フック
(variable_declarator
  name: (object_pattern
    [
      (pair_pattern
        key: (property_identifier) @use_translation_t (#match? @use_translation_t "^t$")
        value: (identifier) @i18n.trans_func_name
      )
      (shorthand_property_identifier_pattern) @i18n.trans_func_name
    ]
  )
  value: (call_expression
    function: (identifier) @use_translation (#match? @use_translation "^useTranslation$")
    arguments: (arguments
      [
        (string (string_fragment) @i18n.namespace)
        (array)
        (undefined)
      ]?
      (object
        (pair
          key: (property_identifier) @key_prefix_key (#match? @key_prefix_key "^keyPrefix$")
          value: (string (string_fragment) @i18n.key_prefix)
        )?
      )?
    )
  )
) @i18n.get_trans_func

;; Trans コンポーネント（自己終了タグ）
(jsx_self_closing_element
  name: (identifier) @trans (#match? @trans "^Trans$")
  attribute: (jsx_attribute
    (property_identifier) @i18n_key (#match? @i18n_key "^i18nKey$")
    [
      (string (string_fragment) @i18n.key) @i18n.key_arg
      (jsx_expression
        (string (string_fragment) @i18n.key) @i18n.key_arg
      )
    ]
  )
) @i18n.call_trans_func

;; Trans コンポーネント（開始タグ）
(jsx_element
  open_tag: (jsx_opening_element
    name: (identifier) @trans (#match? @trans "^Trans$")
    attribute: (jsx_attribute
      (property_identifier) @i18n_key (#match? @i18n_key "^i18nKey$")
      [
        (string (string_fragment) @i18n.key) @i18n.key_arg
        (jsx_expression
          (string (string_fragment) @i18n.key) @i18n.key_arg
        )
      ]
    )
  )
) @i18n.call_trans_func