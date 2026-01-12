;; Call useTranslation
(variable_declarator
  name: (object_pattern
    [
      (pair_pattern
        key: (property_identifier) @use_translation_t (#eq? @use_translation_t "t")
        value: (identifier) @i18n.get_trans_fn_name
      )
      (shorthand_property_identifier_pattern) @i18n.get_trans_fn_name
    ]
    )
  value:
    (call_expression
      function: (identifier) @use_translation (#eq? @use_translation "useTranslation")
      arguments: (arguments
        [
          (string (string_fragment) @i18n.namespace)
          (array (string (string_fragment) @i18n.namespace_item))
          (undefined)
        ]?
        (object
          (pair
            key: (property_identifier) @key_prefix_key (#eq? @key_prefix_key "keyPrefix")
            value: (string (string_fragment) @i18n.trans_key_prefix)
          )?
        )?
      )
    )
) @i18n.get_trans_fn

;; Call t(translation) function
(call_expression
  function: [
    (identifier) @i18n.call_trans_fn_name
    (member_expression) @i18n.call_trans_fn_name
  ]
    arguments: (arguments
      (string
        (string_fragment)? @i18n.trans_key
      )? @i18n.trans_key_arg
      (object
        (pair
          key: (property_identifier) @ns_key (#eq? @ns_key "ns")
          value: (string (string_fragment) @i18n.explicit_namespace)
        )?
      )?
    ) @i18n.trans_args
) @i18n.call_trans_fn

;; Translation コンポーネント
(jsx_element
  open_tag: (jsx_opening_element
    name: (identifier) @translation (#eq? @translation "Translation")
    attribute: (jsx_attribute
      (property_identifier) @key_prefix_attr (#eq? @key_prefix_attr "keyPrefix")
      [
        (string (string_fragment) @i18n.trans_key_prefix)
        (jsx_expression (string (string_fragment) @i18n.trans_key_prefix))
      ]
    )?
  )
  (jsx_expression
    [
      (arrow_function parameters: (formal_parameters (_) @i18n.get_trans_fn_name))
      (function_expression parameters: (formal_parameters (_) @i18n.get_trans_fn_name))
    ]
  )
) @i18n.get_trans_fn

;; Trans コンポーネント (self-closing)
(jsx_self_closing_element
  name: (identifier) @trans (#eq? @trans "Trans")
  attribute: (jsx_attribute
    (property_identifier) @i18n_key (#eq? @i18n_key "i18nKey")
    [
      (string (string_fragment) @i18n.trans_key) @i18n.trans_key_arg
      (jsx_expression (string (string_fragment) @i18n.trans_key) @i18n.trans_key_arg)
    ]
  )
  attribute: (jsx_attribute
    (property_identifier) @attr_t (#eq? @attr_t "t")
    (jsx_expression (identifier) @i18n.call_trans_fn_name)
  )?
) @i18n.call_trans_fn

;; Trans コンポーネント (opening element)
(jsx_opening_element
  name: (identifier) @trans (#eq? @trans "Trans")
  attribute: (jsx_attribute
    (property_identifier) @i18n_key (#eq? @i18n_key "i18nKey")
    [
      (string (string_fragment) @i18n.trans_key) @i18n.trans_key_arg
      (jsx_expression (string (string_fragment) @i18n.trans_key) @i18n.trans_key_arg)
    ]
  )
  attribute: (jsx_attribute
    (property_identifier) @attr_t (#eq? @attr_t "t")
    (jsx_expression (identifier) @i18n.call_trans_fn_name)
  )?
) @i18n.call_trans_fn
