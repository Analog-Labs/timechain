use alloy::{
	providers::{EthGetBlockParams, Provider, ProviderCall},
	rpc::{
		json_rpc::{RpcRecv, RpcSend},
		types::{Block, Header},
	},
};
use futures::{FutureExt, Stream};
use std::{
	future::IntoFuture,
	pin::Pin,
	task::{Context, Poll},
};
use tokio::time::{sleep, Duration, Instant, Sleep};

/// Default polling interval for checking for new finalized blocks.
const DEFAULT_POLLING_INTERVAL: Duration = Duration::from_secs(2);
/// Minimal polling interval (500ms)
const MIN_POLLING_INTERVAL: Duration = Duration::from_millis(500);
/// Max polling interval (1 minute)
const MAX_POLLING_INTERVAL: Duration = Duration::from_secs(60);
/// Default adjust factor, used for tune the polling interval.
const ADJUST_FACTOR: Duration = Duration::from_millis(500);
/// The threshold to adjust the polling interval.
const ADJUST_THRESHOLD: i32 = 10;

/// State machine that delays invoking future until delay is elapsed.
enum StateMachine<P: RpcSend, R: RpcRecv> {
	/// Waiting for the polling interval to elapse.
	Wait(Pin<Box<Sleep>>),
	/// Fetching the latest finalized block.
	Polling(ProviderCall<P, R>),
}

/// Statistics to dynamically adjust the polling interval.
struct Statistics {
	/// Latest known finalized block.
	best_finalized_block: Option<Header>,
	/// Incremented the best finalized block is parent of the new block.
	/// Ex: if the best known finalized block is 100, and the new block is 101.
	new: u32,
	/// Counts how many times the backend returned the same finalized block.
	/// Ex: if the best known finalized block is 100, and the new block is 100.
	duplicated: u32,
	/// Incremented when the new finalized block is not parent of the last known finalized block.
	/// Ex: if the best known finalized block is 100, and the new block is 105.
	gaps: u32,
	/// Controls when the polling interval should be updated.
	adjust_threshold: i32,
	/// polling interval for check for new finalized blocks. adjusted dynamically.
	polling_interval: Duration,
}

impl Statistics {
	/// Updates the statistics with the new finalized block.
	fn on_finalized_block(&mut self, new_block: &Header) -> bool {
		let Some(best_finalized_block) = self.best_finalized_block.as_ref() else {
			self.best_finalized_block = Some(new_block.clone());
			return true;
		};
		if new_block.number < best_finalized_block.number {
			tracing::warn!(
				"Non monotonically increasing finalized number, best: {}, received: {}",
				best_finalized_block.number,
				new_block.number
			);
			return false;
		}
		// Update the adjust factor, this formula converges to equalize the ratio of duplicated and
		// ratio of gaps.
		let expected = best_finalized_block.number + 1;
		let is_valid = if new_block.number == best_finalized_block.number {
			self.duplicated += 1;
			self.adjust_threshold -= 1;
			false
		} else if new_block.number == expected {
			self.new += 1;
			true
		} else {
			debug_assert!(
				new_block.number > expected,
				"Non monotonically increasing finalized block number"
			);
			// Cap the gap_size to `ADJUST_THRESHOLD`.
			let gap_size =
				i32::try_from(new_block.number - expected).unwrap_or(1).min(ADJUST_THRESHOLD);
			self.gaps += 1;
			self.adjust_threshold -= gap_size;
			true
		};
		// Adjust the polling interval
		if self.adjust_threshold >= ADJUST_THRESHOLD {
			// Increment the polling interval by `ADJUST_FACTOR`
			self.adjust_threshold -= ADJUST_THRESHOLD;
			self.polling_interval += ADJUST_FACTOR;
			self.polling_interval = self.polling_interval.saturating_add(ADJUST_FACTOR);
		} else if self.adjust_threshold <= -ADJUST_THRESHOLD {
			// Decrement the polling interval by `ADJUST_FACTOR`
			self.adjust_threshold += ADJUST_THRESHOLD;
			self.polling_interval = self.polling_interval.saturating_sub(ADJUST_FACTOR);
		}
		// Clamp the polling interval to guarantee it's within the limits.
		self.polling_interval =
			self.polling_interval.clamp(MIN_POLLING_INTERVAL, MAX_POLLING_INTERVAL);
		// Update the best finalized block.
		if is_valid {
			self.best_finalized_block = Some(new_block.clone());
		}
		is_valid
	}
}

/// A stream which emits new blocks finalized blocks, it also guarantees new finalized blocks are
/// monotonically increasing.
pub struct FinalizedBlockStream<P: Provider> {
	/// Ethereum RPC backend.
	provider: P,
	/// Controls the polling interval for checking for new finalized blocks.
	statistics: Statistics,
	/// Latest known finalized block and the timestamp when it was received.
	best_finalized_block: Option<(Block, Instant)>,
	/// State machine that controls fetching the latest finalized block.
	state: Option<StateMachine<EthGetBlockParams, Option<Block>>>,
	/// Count of consecutive errors.
	consecutive_errors: u32,
}

impl<P> FinalizedBlockStream<P>
where
	P: Provider + Send + 'static,
{
	pub fn new(provider: P) -> Self {
		Self {
			provider,
			statistics: Statistics {
				best_finalized_block: None,
				new: 0,
				duplicated: 0,
				gaps: 0,
				adjust_threshold: 0,
				polling_interval: DEFAULT_POLLING_INTERVAL,
			},
			best_finalized_block: None,
			state: Some(StateMachine::Wait(Box::pin(sleep(Duration::from_millis(1))))),
			consecutive_errors: 0,
		}
	}
}

impl<P> Stream for FinalizedBlockStream<P>
where
	P: Provider + Unpin + Send + 'static,
{
	type Item = Block;

	#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		// Fetch latest finalized block
		loop {
			let Some(state) = self.state.take() else {
				// Safety: the state is always Some, this is unreachable.
				unreachable!(
					"[report this bug] the finalzed block stream state should never be None"
				);
			};
			self.state = match state {
				////////////////////////////////////////////////
				// Waiting for the polling interval to elapse //
				////////////////////////////////////////////////
				StateMachine::Wait(mut delay) => match delay.poll_unpin(cx) {
					Poll::Ready(()) => {
						// TODO
						let provider_call = self
							.provider
							.get_block_by_number(alloy::eips::BlockNumberOrTag::Finalized)
							.into_future();
						Some(StateMachine::Polling(provider_call))
					},
					Poll::Pending => {
						self.state = Some(StateMachine::Wait(delay));
						return Poll::Pending;
					},
				},
				//////////////////////////////////////////
				// Fetching the latest finalized block. //
				//////////////////////////////////////////
				StateMachine::Polling(mut fut) => match fut.poll_unpin(cx) {
					// Backend returned a new finalized block.
					Poll::Ready(Ok(Some(block))) => {
						// Update last finalized block.
						if self.statistics.on_finalized_block(&block.header) {
							self.best_finalized_block = Some((block.clone(), Instant::now()));
							self.state = Some(StateMachine::Wait(Box::pin(sleep(
								self.statistics.polling_interval,
							))));
							return Poll::Ready(Some(block));
						}
						self.consecutive_errors = 0;
						Some(StateMachine::Wait(Box::pin(sleep(self.statistics.polling_interval))))
					},
					// Backend returned None, retry after delay.
					Poll::Ready(Ok(None)) => {
						let delay = self.statistics.polling_interval;
						tracing::warn!(
							"failed to retrieve finalized block, retrying in {delay:?}: None"
						);
						Some(StateMachine::Wait(Box::pin(sleep(delay))))
					},
					// Backend returned an error, retry after delay.
					Poll::Ready(Err(err)) => {
						let delay = self.statistics.polling_interval;
						tracing::warn!(
							"failed to retrieve finalized block, retrying in {delay:?}: {err:?}"
						);
						Some(StateMachine::Wait(Box::pin(sleep(delay))))
					},

					// Request is pending..
					Poll::Pending => {
						self.state = Some(StateMachine::Polling(fut));
						return Poll::Pending;
					},
				},
			}
		}
	}
}
