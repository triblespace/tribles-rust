use crate::trible::*;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variable(usize);

#[derive(Copy, Clone)]
pub struct VariableProposal {
    pub variable: Variable,
    pub count: usize,
    pub forced: bool,
}

#[derive(Copy, Clone)]
pub struct PushResult {
    pub relevant: bool,
    pub done: bool,
}

pub trait Constraint {
    fn propose(&self) -> VariableProposal;
    fn push(&mut self, variable: Variable, ascending: bool) -> PushResult;
    fn pop(&mut self);
    fn valid(&self) -> bool;
    fn peek(&self) -> Segment;
    fn next(&mut self);
    fn seek(&mut self, value: Segment) -> bool;
}

pub struct ConstantConstraint {
    variable: Variable,
    constant: Segment,
    ascending: bool,
    valid: bool,
}

impl Constraint for ConstantConstraint {
    fn propose(&self) -> VariableProposal {
        return VariableProposal {
            variable: self.variable,
            count: 1,
            forced: false,
        };
    }
    fn push(&mut self, variable: Variable, ascending: bool) -> PushResult {
        if variable != self.variable {
            return PushResult {
                relevant: false,
                done: false,
            };
        }
        self.ascending = ascending;
        return PushResult {
            relevant: true,
            done: true,
        };
    }
    fn pop(&mut self) {
        self.valid = true;
    }
    fn valid(&self) -> bool {
        return self.valid;
    }
    fn peek(&self) -> Segment {
        return self.constant;
    }
    fn next(&mut self) {
        self.valid = false;
    }
    fn seek(&mut self, value: Segment) -> bool {
        if self.constant == value {
            return true;
        }
        if self.ascending {
            if self.constant < value {
                self.valid = false;
            }
        } else {
            if self.constant > value {
                self.valid = false;
            }
        }
        return false;
    }
}

pub fn constant_constraint(variable: Variable, constant: Segment) -> Box<dyn Constraint> {
    return Box::new(ConstantConstraint {
        variable,
        constant,
        ascending: false,
        valid: true,
    });
}

/*

class CollectionConstraint {
  constructor(variable1, variable2, collection) {
    this.variable1 = variable1;
    this.variable2 = variable2;
    this.pushed = false;

    this.C1 = emptyPART;
    for (const [c1, c2] of collection) {
      this.C1 = this.C1.put(c1, (C2 = emptyPART) => C2.put(c2));
    }

    this.cursorC1 = null;
    this.cursorC2 = null;
  }

  propose() {
    if (!this.pushed) {
      return {
        variable: this.variable1,
        count: this.C1.count,
        forced: false,
      };
    } else {
      return {
        variable: this.variable2,
        count: this.cursorC1.value().count,
        forced: false,
      };
    }
  }

  push(variable, ascending) {
    if (!this.pushed) {
      if (variable !== this.variable1) return { relevant: false, done: false };
      this.cursorC1 = this.C1.cursor(ascending);
      return { relevant: true, done: false };
    } else {
      if (variable !== this.variable2) return { relevant: false, done: false };
      this.cursorC1 = this.cursorC1.value().cursor(ascending);
      return { relevant: true, done: true };
    }
  }

  pop() {
    this.pushed = false;
  }

  valid() {
    if (!this.pushed) {
      return this.cursorC1.valid;
    } else {
      return this.cursorC2.valid;
    }
  }

  peek() {
    if (!this.pushed) {
      return this.cursorC1.peek();
    } else {
      return this.cursorC2.peek();
    }
  }

  next() {
    if (!this.pushed) {
      this.cursorC1.next();
      this.valid = this.cursorC1.valid;
    } else {
      this.cursorC2.next();
      this.valid = this.cursorC2.valid;
    }
  }

  seek(value) {
    if (!this.pushed) {
      const match = this.cursorC1.seek(value);
      this.valid = this.cursorC1.valid;
      return match;
    } else {
      const match = this.cursorC2.seek(value);
      this.valid = this.cursorC2.valid;
      return match;
    }
  }
}

class TripleConstraint {
  constructor(db, variableE, variableA, variableV1, variableV2) {
    this.branch = db;
    this.variableE = variableE;
    this.variableA = variableA;
    this.variableV1 = variableV1;
    this.variableV2 = variableV2;
    this.cursors = [];
  }

  propose() {
    let branch;
    if (this.cursors.length === 0) {
      branch = this.db.index;
    } else {
      branch = this.cursors[this.cursors.length - 1].value();
    }

    let count = Number.MAX_VALUE;
    let index;
    let variable;

    if (branch.V2) {
      return {
        variable: this.variableV2,
        count: branch.V2.count,
        forced: true,
      };
    }
    if (branch.E && branch.E.count <= count) {
      count = branch.E.count;
      index = branch.E;
      variable = this.variableE;
    }
    if (branch.A && branch.A.count <= count) {
      count = branch.A.count;
      index = branch.A;
      variable = this.variableA;
    }
    if (branch.V1 && branch.V1.count <= count) {
      count = branch.V1.count;
      index = branch.V1;
      variable = this.variableV1;
    }
    return {
      variable,
      count,
      forced: false,
    };
  }

  push(variable, ascending = true) {
    let branch;
    if (this.cursors.length === 0) {
      branch = this.db.index;
    } else {
      branch = this.cursors[this.cursors.length - 1].value();
    }

    const done = this.cursors.length === 3;
    if (variable === this.variableE) {
      this.cursors.push(branch.E.cursor(ascending));
      return { relevant: true, done };
    }
    if (variable === this.variableA) {
      this.cursors.push(branch.A.cursor(ascending));
      return { relevant: true, done };
    }
    if (variable === this.variableV1) {
      this.cursors.push(branch.V1.cursor(ascending));
      return { relevant: true, done };
    }
    if (variable === this.variableV2) {
      this.cursors.push(branch.V2.cursor(ascending));
      return { relevant: true, done };
    }
    return { relevant: false, done };
  }

  pop() {
    this.cursors.pop();
  }

  valid() {
    this.cursors[this.cursors.length - 1].valid;
  }

  peek() {
    if (this.cursors[this.cursors.length - 1].valid) {
      return this.cursor.peek();
    }
    return null;
  }

  next() {
    this.cursors[this.cursors.length - 1].next();
  }

  seek(value) {
    return this.cursors[this.cursors.length - 1].seek(value);
  }
}

//TODO class VariableOrderConstraint {

function* resolve(constraints, ascendingVariables, bindings = new Map()) {
  //init
  let candidateVariable;
  let candidateCount = Number.MAX_VALUE;
  for (const c of constraints) {
    const proposal = c.propose();
    if (proposal.count === 0) {
      return;
    }
    if (!proposal.forced) {
      candidateVariable = proposal.variable;
      break;
    }
    if (proposal.count <= candidateCount) {
      candidateVariable = proposal.variable;
      candidateCount = proposal.count;
    }
  }

  const ascending = ascendingVariables.has(candidateVariable);

  const restConstraints = [];
  const currentConstraints = [];
  for (const c of constraints) {
    const pushed = c.push(ascending);
    if (!pushed.done) {
      restConstraints.push(c);
    }
    if (pushed.relevant) {
      currentConstraints.push(c);
    }
  }

  const lastVariable = restConstraints.length === 0;

  let candidateOrigin = 0;
  let candidate = currentConstraints[candidateOrigin].peek();
  let i = candidateOrigin;
  while (true) {
    i = (candidateOrigin + 1) % currentConstraints.length;
    if (i === candidateOrigin) {
      bindings[candidateVariable] = candidate;
      if (lastVariable) {
        yield bindings;
      } else {
        yield* resolve(restConstraints, ascendingVariables, bindings);
      }
      currentConstraints[candidateOrigin].next();
      if (!currentConstraints[candidateOrigin].valid) break;
      candidate = currentConstraints[candidateOrigin].peek();
    } else {
      const match = currentConstraints[i].seek(candidate);
      if (!currentConstraints[i].valid) break;
      if (!match) {
        candidateOrigin = i;
        candidate = currentConstraints[i].peek();
      }
    }
  }

  currentConstraints.forEach((c) => c.pop());
  return;
}

export {
  CollectionConstraint,
  ConstantConstraint,
  IndexConstraint,
  query,
  TripleConstraint,
};
*/
