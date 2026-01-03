//! スコープ管理モジュール
use std::collections::HashMap;

use tree_sitter::Node;

use super::types::GetTransFnDetail;

/// スコープ情報を保持する構造体
#[derive(Debug, Clone)]
pub struct ScopeInfo<'a> {
    /// スコープを定義するノード（関数やコンポーネントなど）
    pub scope_node: Node<'a>,
    /// スコープ内で使用される翻訳関数の詳細情報
    pub trans_fn: GetTransFnDetail,
}

/// スコープ情報の実装
impl<'a> ScopeInfo<'a> {
    /// デフォルトのスコープ情報を作成
    /// # Arguments
    /// * `scope_node` - スコープのノード
    /// * `trans_fn` - スコープ内で使用される翻訳関数
    /// # Returns
    /// * `ScopeInfo` - 作成されたスコープ情報
    #[must_use]
    pub const fn new(scope_node: Node<'a>, trans_fn: GetTransFnDetail) -> Self {
        Self { scope_node, trans_fn }
    }
}

/// スコープ管理を行うクラス
#[derive(Default, Debug)]
pub struct Scopes<'a> {
    /// 翻訳関数名ごとのスコープスタック
    stacks: HashMap<String, Vec<ScopeInfo<'a>>>,
}

/// スコープ管理の実装
impl<'a> Scopes<'a> {
    /// デフォルトのスコープ管理を作成
    #[must_use]
    pub fn new() -> Self {
        Self { stacks: HashMap::new() }
    }

    /// 存在する翻訳関数名の一覧
    /// # Returns
    /// * `impl Iterator<Item = &String>` - アクティブなスコープを持つ `trans_fn_name` のイテレータ
    pub fn trans_fn_names(&self) -> impl Iterator<Item = &String> {
        self.stacks.iter().filter(|(_, stack)| !stack.is_empty()).map(|(name, _)| name)
    }

    /// スコープをプッシュ
    /// # Arguments
    /// * `trans_fn_name` - プッシュするスコープの翻訳関数名
    /// * `scope_info` - プッシュするスコープ情報
    /// # Returns
    /// * `()` - なし
    pub fn push_scope(&mut self, trans_fn_name: String, scope_info: ScopeInfo<'a>) {
        self.stacks.entry(trans_fn_name).or_default().push(scope_info);
    }

    /// スコープをポップ
    /// # Arguments
    /// * `trans_fn_name` - ポップするスコープの翻訳関数名
    /// # Returns
    /// * `Option<ScopeInfo>` - ポップされたスコープ情報、存在しない場合はNone
    pub fn pop_scope(&mut self, trans_fn_name: &str) -> Option<ScopeInfo<'a>> {
        self.stacks.get_mut(trans_fn_name).and_then(Vec::pop)
    }

    /// 現在のスコープを取得（最も内側）
    /// # Arguments
    /// * `trans_fn_name` - 取得するスコープの翻訳関数名
    /// # Returns
    /// * `Option<&ScopeInfo>` - 現在のスコープ情報、 存在しない場合はNone
    #[must_use]
    pub fn current_scope(&self, trans_fn_name: &str) -> Option<&ScopeInfo<'a>> {
        self.stacks.get(trans_fn_name).and_then(|stack| stack.last())
    }

    /// デフォルトのスコープを取得（最初にプッシュされたスコープ）
    /// # Arguments
    /// * `trans_fn_name` - 取得するスコープの翻訳関数名
    /// # Returns
    /// * `Option<&ScopeInfo>` - デフォルトのスコープ情報、 存在しない場合はNone
    #[must_use]
    pub fn default_scope(&self, trans_fn_name: &str) -> Option<&ScopeInfo<'a>> {
        self.stacks.get(trans_fn_name).and_then(|stack| stack.first())
    }

    /// ノードが現在のスコープ内にあるかをチェック
    /// # Arguments
    /// * `trans_fn_name` - チェックするスコープの翻訳関数名
    /// * `node` - チェックするノード
    /// # Returns
    /// * `bool` - ノードがスコープ内にある場合はtrue
    #[must_use]
    pub fn is_node_in_current_scope(&self, trans_fn_name: &str, node: Node<'a>) -> bool {
        self.current_scope(trans_fn_name).is_some_and(|current_scope| {
            let scope_node = current_scope.scope_node;
            node.start_byte() >= scope_node.start_byte() && node.end_byte() <= scope_node.end_byte()
        })
    }

    /// 特定の翻訳関数名のスコープが存在するかをチェック
    /// # Arguments
    /// * `trans_fn_name` - チェックする翻訳関数名
    /// # Returns
    /// * `bool` - スコープが存在する場合はtrue
    #[must_use]
    pub fn has_scope(&self, trans_fn_name: &str) -> bool {
        self.stacks.get(trans_fn_name).map_or_else(|| false, |stack| !stack.is_empty())
    }
}
