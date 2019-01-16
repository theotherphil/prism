
use std::collections::HashSet;
use crate::syntax::{Func, Schedule};

/// Doesn't yet look very graph-like...
pub struct Graph {
    pub name: String,
    funcs: Vec<Func>,
    /// Names of the required input buffers,
    /// computed from funcs.
    inputs: Vec<String>,
    /// Names of the output buffers (including)
    /// all intermediates), in some valid dependency
    /// order.
    outputs: Vec<String>,
    /// Names of the required parameters,
    /// computed form funcs. These are guaranteed to be
    /// in lexicographic order.
    params: Vec<String>,
    /// The iteration, compute and storage orders for each function.
    pub(crate) schedule: Schedule
}

impl Graph {
    pub fn new(name: &str, funcs: Vec<Func>, schedule: Schedule) -> Graph {
        let name = name.to_string();
        // The names of the funcs being computed
        let func_names: HashSet<String> = funcs.iter().map(|f| f.name.clone()).collect();
        // The buffers that any func reads from
        let reads: HashSet<String> = funcs.iter().flat_map(|f| f.sources()).collect();
        // The buffers that are read from but not
        // computed and so must be provided as inputs
        let mut inputs: Vec<String> = reads.difference(&func_names).cloned().collect();
        inputs.sort();
        // TODO: actually do the topological sort!
        // TODO: for now we just assume that the inputs were provided in a valid order
        let outputs: Vec<String> = funcs.iter().map(|f| f.name.clone()).collect();

        let params: HashSet<String> = funcs.iter().flat_map(|f| f.params()).collect();
        let mut params: Vec<String> = params.iter().cloned().collect();
        params.sort();

        let schedule_names: HashSet<String> = schedule.func_schedules.keys().cloned().collect();
        let funcs_without_schedules: Vec<String> = func_names.difference(&schedule_names).cloned().collect();
        let buffers: HashSet<String> = inputs.iter().cloned().collect();
        let buffers_without_schedules: Vec<String> = buffers.difference(&schedule_names).cloned().collect();
        if funcs_without_schedules.len() > 0 || buffers_without_schedules.len() > 0 {
            let schedule_names: Vec<String> = schedule_names.iter().cloned().collect();
            panic!("Not all functions have schedules. \
                Schedule names: {}, Missing funcs: {}. Missing buffers: {}",
                schedule_names.join(", "),
                funcs_without_schedules.join(", "),
                buffers_without_schedules.join(", ")
            );
        }

        Graph { name, funcs, inputs, outputs, params, schedule }
    }

    pub fn funcs(&self) -> &[Func] {
        &self.funcs
    }

    pub fn inputs(&self) -> &[String] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[String] {
        &self.outputs
    }

    pub fn input_then_outputs(&self) -> Vec<String> {
        self.inputs().iter().chain(self.outputs()).cloned().collect()
    }

    pub fn params(&self) -> &[String] {
        &self.params
    }
}
