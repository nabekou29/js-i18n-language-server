;; useI18n() destructuring: const { t, te, tm } = useI18n()
;; Registers destructured variables as translation functions (GetTransFn).
(variable_declarator
  name: (object_pattern
    [
      (pair_pattern
        key: (property_identifier) @_use_i18n_key (#match? @_use_i18n_key "^(t|te|tm)$")
        value: (identifier) @i18n.get_trans_fn_name
      )
      (shorthand_property_identifier_pattern) @i18n.get_trans_fn_name
        (#match? @i18n.get_trans_fn_name "^(t|te|tm)$")
    ]
  )
  value:
    (call_expression
      function: (identifier) @_use_i18n (#eq? @_use_i18n "useI18n")
    )
) @i18n.get_trans_fn

;; useI18n() assigned to variable: const i18n = useI18n(); i18n.t('key')
;; The variable itself is registered so i18n.t() is recognized via member expression.
(variable_declarator
  name: (identifier) @i18n.get_trans_fn_name
  value:
    (call_expression
      function: (identifier) @_use_i18n_obj (#eq? @_use_i18n_obj "useI18n")
    )
) @i18n.get_trans_fn

;; this.$t('key'), this.$tc('key'), this.$te('key'), this.$tm('key')
;; Member expression on `this` with vue-i18n global functions.
(call_expression
  function:
    (member_expression
      object: (this) @_this
      property: (property_identifier) @i18n.call_trans_fn_name
        (#match? @i18n.call_trans_fn_name "^\\$(t|tc|te|tm)$")
    )
  arguments: (arguments
    (string
      (string_fragment)? @i18n.trans_key
    )? @i18n.trans_key_arg
    (_)*
  ) @i18n.trans_args
) @i18n.call_trans_fn
