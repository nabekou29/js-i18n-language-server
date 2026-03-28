;; svelte-i18n object form: $_({ id: 'key', ... })
(call_expression
  function: (identifier) @i18n.call_trans_fn_name
    (#match? @i18n.call_trans_fn_name "^\\$(_|t|format|json)$")
  arguments: (arguments
    (object
      (pair
        key: (property_identifier) @_id_key (#eq? @_id_key "id")
        value: (string (string_fragment) @i18n.trans_key) @i18n.trans_key_arg
      )
    )
  ) @i18n.trans_args
) @i18n.call_trans_fn
