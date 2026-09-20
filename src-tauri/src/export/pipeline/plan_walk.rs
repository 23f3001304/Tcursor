use crate::export::remap::ClipSpan;

pub struct PlanCursor {
    plan: Vec<u64>,
    j: usize,
    decoded: Option<u64>,
    base: u64,
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
            base: 0,
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
            None => k.saturating_sub(self.base) + 1,
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

    pub fn rebase(&mut self, base_k: u64) {
        self.base = base_k;
        self.decoded = None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipJoin {
    pub first_k: u64,
    pub prev_clip: usize,
    pub respawn: bool,
}

pub fn join_at(spans: &[ClipSpan], plan: &[u64], j: usize) -> Option<ClipJoin> {
    let i = spans.iter().position(|s| s.plan_start == j)?;
    let prev_clip = spans.get(i.checked_sub(1)?)?.clip;
    let prev_k = *plan.get(j.checked_sub(1)?)?;
    Some(ClipJoin {
        first_k: spans[i].first_k,
        prev_clip,
        respawn: spans[i].first_k <= prev_k,
    })
}

#[cfg(test)]
#[path = "plan_walk_tests.rs"]
mod tests;
