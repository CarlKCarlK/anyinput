// cmk rename to input-like-derive (or derive-input-like)
// cmk remove cargo stuff features of syn no longer needed.
// cmk create unique names for macros with gensym
// cmk look a syn's macro workshop examples

// Parsing based on https://github.com/dtolnay/syn/tree/master/examples/trace-var
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    fold::{self, Fold},
    parse_macro_input, Expr, ItemFn, Stmt,
};

struct Args {}

impl Fold for Args {
    fn fold_expr(&mut self, e: Expr) -> Expr {
        match e {
            // Expr::Assign(e) => {
            //     // if self.should_print_expr(&e.left) {
            //     //     self.assign_and_print(*e.left, &e.eq_token, *e.right)
            //     // } else {
            //     //     Expr::Assign(fold::fold_expr_assign(self, e))
            //     // }
            //     panic!("Assign not supported");
            // }
            // Expr::AssignOp(e) => {
            //     // if self.should_print_expr(&e.left) {
            //     //     self.assign_and_print(*e.left, &e.op, *e.right)
            //     // } else {
            //     //     Expr::AssignOp(fold::fold_expr_assign_op(self, e))
            //     // }
            //     panic!("AssignOp not supported");
            // }
            _ => fold::fold_expr(self, e),
        }
    }

    fn fold_stmt(&mut self, s: Stmt) -> Stmt {
        match s {
            // Stmt::Local(s) => {
            //     // if s.init.is_some() && self.should_print_pat(&s.pat) {
            //     //     self.let_and_print(s)
            //     // } else {
            //     //     Stmt::Local(fold::fold_local(self, s))
            //     // }
            //     panic!("Local not supported");
            // }
            _ => fold::fold_stmt(self, s),
        }
    }
}

#[proc_macro_attribute]
pub fn input_like(_: TokenStream, input: TokenStream) -> TokenStream {
    // cmk 0 item
    // panic!("item: {:#?}", &item);
    // item

    let input = parse_macro_input!(input as ItemFn);
    // panic!("item: {:#?}", &input);

    let mut args = Args {};
    let output = args.fold_item_fn(input);

    TokenStream::from(quote!(#output))
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        // cmk update tests
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
