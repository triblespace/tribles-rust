use super::*;

pub struct MaskConstraint<'a> {
    mask: VariableSet,
    constraint: Box<dyn Constraint<'a> + 'a>,
}

impl<'a> MaskConstraint<'a> {
    pub fn new(mask: VariableSet, constraint: Box<dyn Constraint<'a> + 'a>) -> Self {
        MaskConstraint { mask, constraint }
    }
}

impl<'a> Constraint<'a> for MaskConstraint<'a> {
    fn variables(&self) -> VariableSet {
        self.constraint.variables().intersect(self.mask)
    }

    fn variable(&self, variable: VariableId) -> bool {
        self.mask.is_set(variable) && self.constraint.variable(variable)
    }

    fn estimate(&self, variable: VariableId, binding: &Binding) -> usize {
        self.constraint.estimate(variable, binding)
    }

    fn propose(&self, variable: VariableId, binding: &Binding) -> Vec<Value> {
        self.constraint.propose(variable, binding)
    }

    fn confirm(&self, variable: VariableId, binding: &Binding, proposals: &mut Vec<Value>) {
        self.constraint.confirm(variable, binding, proposals)
    }
}

#[macro_export]
macro_rules! mask {
    ($ctx:expr, ($($Var:ident),+), $c:expr) => (
        {
            let mut mask = $crate::query::VariableSet::new_empty();
            $(let $Var = $ctx.next_variable();
              mask.set($Var.index);)*
            $crate::query::MaskConstraint::new(mask, Box::new($c))
        }
    )
}

pub use mask;
