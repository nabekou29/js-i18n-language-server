;; getFixedT 関数呼び出し
;; 引数: (lang, ns?, keyPrefix?)
(variable_declarator
  name: (identifier) @i18n.get_trans_fn_name
  value:
    (call_expression
      function: [
        (identifier)
        (member_expression)
      ] @get_fixed_t_func (#match? @get_fixed_t_func "getFixedT$")
      arguments: (arguments) @i18n.get_trans_fn_args
    )
) @i18n.get_trans_fn
