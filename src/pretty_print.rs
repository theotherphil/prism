
pub trait PrettyPrint {
    fn pretty_print(&self) -> String;
    fn is_leaf(&self) -> bool;
}

pub fn combine_with_op<P: PrettyPrint>(op: &str, left: &P, right: &P) -> String {
    let left = pretty_print_with_parens(left);
    let right = pretty_print_with_parens(right);
    format!("{} {} {}", left, op, right)
}

pub fn pretty_print_with_parens<P: PrettyPrint>(p: &P) -> String {
    let pp = p.pretty_print();
    if p.is_leaf() { pp } else { format!("({})", pp) }
}

impl<P: PrettyPrint> PrettyPrint for Box<P> {
    fn pretty_print(&self) -> String {
        (**self).pretty_print()
    }

    fn is_leaf(&self) -> bool {
        (**self).is_leaf()
    }
}
