use syn::{ExprMatch, visit::Visit};

pub struct MatchFinder<'ast> {
    pub found: Option<&'ast ExprMatch>,
}

impl<'ast> Visit<'ast> for MatchFinder<'ast> {
    fn visit_expr_match(&mut self, node: &'ast ExprMatch) {
        if self.found.is_none() {
            self.found = Some(node);
            // Don't delegate to the default impl — stops recursion into nested
            // matches inside arms, so we always capture the outermost one.
        }
    }
}
