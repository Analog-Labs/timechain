use prometheus_exporter::prometheus::{
	core::{AtomicF64, GenericGaugeVec},
	register_gauge_vec,
};
use time_primitives::{Function, TaskPhase};

/// Prometheus counter metric for chronicle tasks. It records the
/// the number of tasks per (task_phase,function) that the
/// chronicle is currently running.
#[derive(Clone, Debug)]
pub struct TaskPhaseCounter {
	gauge: Option<GenericGaugeVec<AtomicF64>>,
}

impl TaskPhaseCounter {
	pub fn new() -> Self {
		let gauge_result = register_gauge_vec!(
			"chronicle_task_count",
			"Number of tasks in the chronicle queue",
			&["phase", "function"]
		);

		// If for some reason the registry declines the metric, log it and do nothing
		// on metric updates
		let gauge = match gauge_result {
			Ok(c) => Some(c),
			Err(err) => {
				tracing::warn!("TaskCounterMetric failed to register: {}", err);
				None
			},
		};

		Self { gauge }
	}

	pub fn set(&self, phase: &TaskPhase, function: &Function, value: f64) {
		match self.gauge.as_ref() {
			Some(g) => 
				g.with_label_values(&[&phase.to_string(), &function.to_string()])
				.set(value),
			None => (),
		}
	}

	pub fn inc(&self, phase: &TaskPhase, function: &Function) {
		match self.gauge.as_ref() {
			Some(g) => 
				g.with_label_values(&[&phase.to_string(), &function.to_string()])
				.inc(),
			None => (),
		}
	}

	pub fn dec(&self, phase: &TaskPhase, function: &Function) {
		match self.gauge.as_ref() {
			Some(g) => 
				g.with_label_values(&[&phase.to_string(), &function.to_string()])
				.dec(),
			None => (),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	pub fn duplicate_metrics_dont_panic() {
		let metric1 = TaskPhaseCounter::new();
		let metric2 = TaskPhaseCounter::new();

		metric1.inc(&TaskPhase::Read, &Function::ReadMessages);
		metric2.inc(&TaskPhase::Read, &Function::ReadMessages);

		metric1.inc(&TaskPhase::Sign, &Function::ReadMessages);
		metric2.inc(&TaskPhase::Write, &Function::RegisterShard { shard_id: 2 });

		metric1.dec(&TaskPhase::Read, &Function::ReadMessages);
		metric2.dec(&TaskPhase::Write, &Function::RegisterShard { shard_id: 2 });
	}
}