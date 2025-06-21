;; 基本的なt関数呼び出し - t("key")
(call_expression
  function: (identifier) @func_name (#match? @func_name "^t$")
  arguments: (arguments
    (string
      (string_fragment) @i18n.key
    ) @i18n.key_arg
  )
) @i18n.call_trans_func

;; i18n.t("key") パターン
(call_expression
  function: (member_expression
    object: (identifier) @obj_name (#match? @obj_name "^i18n(ext)?$")
    property: (property_identifier) @prop_name (#match? @prop_name "^t$")
  ) @func_name
  arguments: (arguments
    (string
      (string_fragment) @i18n.key
    ) @i18n.key_arg
  )
) @i18n.call_trans_func

;; translate("key") パターン
(call_expression
  function: (identifier) @func_name (#match? @func_name "^translate$")
  arguments: (arguments
    (string
      (string_fragment) @i18n.key
    ) @i18n.key_arg
  )
) @i18n.call_trans_func

;; i18n.translate("key") パターン
(call_expression
  function: (member_expression
    object: (identifier) @obj_name (#match? @obj_name "^i18n(ext)?$")
    property: (property_identifier) @prop_name (#match? @prop_name "^translate$")
  ) @func_name
  arguments: (arguments
    (string
      (string_fragment) @i18n.key
    ) @i18n.key_arg
  )
) @i18n.call_trans_func