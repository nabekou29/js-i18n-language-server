//! Scope management for translation functions.

use std::collections::HashMap;

use tree_sitter::Node;

use super::types::GetTransFnDetail;

/// Holds scope information for a translation function.
#[derive(Debug, Clone)]
pub struct ScopeInfo<'a> {
    pub scope_node: Node<'a>,
    pub trans_fn: GetTransFnDetail,
}

impl<'a> ScopeInfo<'a> {
    #[must_use]
    pub const fn new(scope_node: Node<'a>, trans_fn: GetTransFnDetail) -> Self {
        Self { scope_node, trans_fn }
    }
}

/// Manages scope stacks per translation function name.
#[derive(Default, Debug)]
pub struct Scopes<'a> {
    stacks: HashMap<String, Vec<ScopeInfo<'a>>>,
}

impl<'a> Scopes<'a> {
    #[must_use]
    pub fn new() -> Self {
        Self { stacks: HashMap::new() }
    }

    pub fn trans_fn_names(&self) -> impl Iterator<Item = &String> {
        self.stacks.iter().filter(|(_, stack)| !stack.is_empty()).map(|(name, _)| name)
    }

    pub fn push_scope(&mut self, trans_fn_name: String, scope_info: ScopeInfo<'a>) {
        self.stacks.entry(trans_fn_name).or_default().push(scope_info);
    }

    pub fn pop_scope(&mut self, trans_fn_name: &str) -> Option<ScopeInfo<'a>> {
        self.stacks.get_mut(trans_fn_name).and_then(Vec::pop)
    }

    #[must_use]
    pub fn current_scope(&self, trans_fn_name: &str) -> Option<&ScopeInfo<'a>> {
        self.stacks.get(trans_fn_name).and_then(|stack| stack.last())
    }

    #[must_use]
    pub fn is_node_in_current_scope(&self, trans_fn_name: &str, node: Node<'a>) -> bool {
        self.current_scope(trans_fn_name).is_some_and(|current_scope| {
            let scope_node = current_scope.scope_node;
            node.start_byte() >= scope_node.start_byte() && node.end_byte() <= scope_node.end_byte()
        })
    }

    #[must_use]
    pub fn has_scope(&self, trans_fn_name: &str) -> bool {
        self.stacks.get(trans_fn_name).is_some_and(|stack| !stack.is_empty())
    }
}
