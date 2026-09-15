pub struct PlanCursor {
    plan: Vec<u64>,
    j: usize,
    decoded: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub j: u64,
    pub k: u64,
    pub decodes_needed: u64,
}

impl PlanCursor {
    pub fn new(plan: Vec<u64>) -> Self {
        PlanCursor {
            plan,
            j: 0,
            decoded: None,
        }
    }
    pub fn len(&self) -> usize {
        self.plan.len()
    }
    pub fn is_empty(&self) -> bool {
        self.plan.is_empty()
    }

    pub fn next(&mut self) -> Option<Step> {
        let k = *self.plan.get(self.j)?;
        let decodes_needed = match self.decoded {
            None => k + 1,
            Some(d) => k.saturating_sub(d),
        };
        let step = Step {
            j: self.j as u64,
            k,
            decodes_needed,
        };
        self.decoded = Some(k);
        self.j += 1;
        Some(step)
    }
}

#[cfg(test)]
#[path = "plan_walk_tests.rs"]
mod tests;
