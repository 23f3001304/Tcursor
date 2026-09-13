// Pure bookkeeping for feeding sequential decoders along a `TimeMap::frame_plan`: for each output
// frame, how many recording frames the pipes must advance before the frame it shows is the held one
// (0 re-uses the held frame - slow motion). No decoding here, so it is tested to the frame.

pub struct PlanCursor { plan: Vec<u64>, j: usize, decoded: Option<u64> }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// Output frame index.
    pub j: u64,
    /// The recording frame it shows.
    pub k: u64,
    /// Recording frames to decode before this one is the held frame (0 = re-use).
    pub decodes_needed: u64,
}

impl PlanCursor {
    pub fn new(plan: Vec<u64>) -> Self { PlanCursor { plan, j: 0, decoded: None } }
    pub fn len(&self) -> usize { self.plan.len() }
    pub fn is_empty(&self) -> bool { self.plan.is_empty() }

    pub fn next(&mut self) -> Option<Step> {
        let k = *self.plan.get(self.j)?;
        let decodes_needed = match self.decoded { None => k + 1, Some(d) => k.saturating_sub(d) };
        let step = Step { j: self.j as u64, k, decodes_needed };
        self.decoded = Some(k);
        self.j += 1;
        Some(step)
    }
}

#[cfg(test)]
#[path = "plan_walk_tests.rs"]
mod tests;
