;; unwrapFunctionStore: const name = unwrapFunctionStore(store)
;; Registers the variable as a translation function (GetTransFn).
(variable_declarator
  name: (identifier) @i18n.get_trans_fn_name
  value:
    (call_expression
      function: (identifier) @_unwrap (#eq? @_unwrap "unwrapFunctionStore")
    )
) @i18n.get_trans_fn

;; Re-query helper: extracts key from an object with `id` property.
;; Used when re-querying captured nodes from patterns below.
;; Not collected during initial scan (no @i18n.call_trans_fn capture).
(object
  (pair
    key: (property_identifier) @_id_requery (#eq? @_id_requery "id")
    value: (string (string_fragment) @i18n.trans_key) @i18n.trans_key_arg
  )
) @i18n.trans_args

;; svelte-i18n object form: $_({ id: 'key', ... })
;; Captures the argument object as call_trans_fn (re-queried by helper above).
(call_expression
  function: (identifier) @i18n.call_trans_fn_name
    (#match? @i18n.call_trans_fn_name "^\\$(_|t|format|json)$")
  arguments: (arguments
    (object) @i18n.call_trans_fn
  )
)

;; defineMessages({ name: { id: 'key' }, ... })
;; Each inner object is captured as a separate call_trans_fn.
(call_expression
  function: (identifier) @_define_messages (#eq? @_define_messages "defineMessages")
  arguments: (arguments
    (object
      (pair
        value: (object) @i18n.call_trans_fn
      )
    )
  )
)
