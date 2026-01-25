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
      (_)*
    ) @i18n.trans_args
) @i18n.call_trans_fn

;; Capture explicit namespace: t("key", { ns: "namespace" })
(call_expression
  arguments: (arguments
    (string)
    (object
      (pair
        key: (property_identifier) @_ns_key (#eq? @_ns_key "ns")
        value: (string (string_fragment) @i18n.explicit_namespace)
      )
    )
  )
)
