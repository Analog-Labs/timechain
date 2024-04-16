use prometheus_exporter::prometheus::{core::{AtomicF64, GenericGaugeVec}, register_gauge_vec};
use time_primitives::{Function, TaskPhase};

#[derive(Clone, Debug)]
pub struct TaskPhaseCounter {
    gauge: GenericGaugeVec<AtomicF64>
}

impl TaskPhaseCounter {
    pub fn new() -> Self {
        let gauge = register_gauge_vec!(
            "chronicle_task_count",
            "Number of tasks in the chronicle queue",
            &["phase","function"]
        ).unwrap();

        Self {
            gauge
        }
    }

    pub fn set(&self, phase: &TaskPhase, function: &Function, value: f64) {
        self.gauge.with_label_values(&[&phase.to_string(), &function.to_string()]).set(value);
    }

    pub fn inc(&self, phase: &TaskPhase, function: &Function) {
        self.gauge.with_label_values(&[&phase.to_string(), &function.to_string()]).inc();
    }

    pub fn dec(&self, phase: &TaskPhase, function: &Function) {
        self.gauge.with_label_values(&[&phase.to_string(), &function.to_string()]).dec();
    }
}