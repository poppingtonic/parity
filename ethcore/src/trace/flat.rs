// Copyright 2015, 2016 Ethcore (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

use basic_types::LogBloom;
use trace::BlockTraces;
use super::trace::{Trace, Action, Res};

/// Trace localized in vector of traces produced by a single transaction.
///
/// Parent and children indexes refer to positions in this vector.
pub struct FlatTrace {
	/// Index of the parent trace within the same transaction.
	pub parent: Option<usize>,
	/// Indexes of child traces within the same transaction.
	pub children: Vec<usize>,
	/// VM depth.
	pub depth: usize,
	/// Type of action performed by a transaction.
	pub action: Action,
	/// Result of this action.
	pub result: Res,
}

/// Represents all traces produced by a single transaction.
pub struct FlatTransactionTraces(Vec<FlatTrace>);

impl Into<Vec<FlatTrace>> for FlatTransactionTraces {
	fn into(self) -> Vec<FlatTrace> {
		self.0
	}
}

/// Represents all traces produced by transactions in a single block.
pub struct FlatBlockTraces(Vec<FlatTransactionTraces>);

impl From<BlockTraces> for FlatBlockTraces {
	fn from(block_traces: BlockTraces) -> Self {
		let traces: Vec<Trace> = block_traces.into();
		let ordered = traces.into_iter()
			.map(|trace| FlatBlockTraces::flatten(None, 0, trace))
			.map(FlatTransactionTraces)
			.collect();
		FlatBlockTraces(ordered)
	}
}

impl Into<Vec<FlatTransactionTraces>> for FlatBlockTraces {
	fn into(self) -> Vec<FlatTransactionTraces> {
		self.0
	}
}

impl FlatBlockTraces {
	/// Helper function flattening nested tree structure to vector of ordered traces.
	fn flatten(parent_index: Option<usize>, len: usize, trace: Trace) -> Vec<FlatTrace> {
		let mut children = vec![];
		let mut next_index = len + 1;
		let all_subs = trace.subs
			.into_iter()
			.flat_map(|subtrace| {
				let subs = FlatBlockTraces::flatten(Some(len), next_index, subtrace);
				children.push(next_index);
				next_index = next_index + subs.len();
				subs
			})
			.collect::<Vec<_>>();

		let ordered = FlatTrace {
			parent: parent_index,
			children: children,
			depth: trace.depth,
			action: trace.action,
			result: trace.result,
		};

		let mut result = vec![ordered];
		result.extend(all_subs);
		result
	}
}

#[cfg(test)]
mod tests {
	use super::{FlatBlockTraces, FlatTransactionTraces, FlatTrace};
	use util::{U256, Address};
	use trace::trace::{Action, Res, CallResult, Call, CreateResult, Create, Trace};
	use trace::BlockTraces;

	#[test]
	fn test_block_from() {
		let trace = Trace {
			depth: 2,
			action: Action::Call(Call {
				from: Address::from(1),
				to: Address::from(2),
				value: U256::from(3),
				gas: U256::from(4),
				input: vec![0x5]
			}),
			subs: vec![
				Trace {
					depth: 3,
					action: Action::Create(Create {
						from: Address::from(6),
						value: U256::from(7),
						gas: U256::from(8),
						init: vec![0x9]
					}),
					subs: vec![
						Trace {
							depth: 3,
							action: Action::Create(Create {
								from: Address::from(6),
								value: U256::from(7),
								gas: U256::from(8),
								init: vec![0x9]
							}),
							subs: vec![
							],
							result: Res::FailedCreate
						},
						Trace {
							depth: 3,
							action: Action::Create(Create {
								from: Address::from(6),
								value: U256::from(7),
								gas: U256::from(8),
								init: vec![0x9]
							}),
							subs: vec![
							],
							result: Res::FailedCreate
						}
					],
					result: Res::FailedCreate
				},
				Trace {
					depth: 3,
					action: Action::Create(Create {
						from: Address::from(6),
						value: U256::from(7),
						gas: U256::from(8),
						init: vec![0x9]
					}),
					subs: vec![],
					result: Res::FailedCreate,
				}
			],
			result: Res::Call(CallResult {
				gas_used: U256::from(10),
				output: vec![0x11, 0x12]
			})
		};

		let block_traces = FlatBlockTraces::from(BlockTraces::from(vec![trace]));
		let transaction_traces: Vec<FlatTransactionTraces> = block_traces.into();
		assert_eq!(transaction_traces.len(), 1);
		let ordered_traces: Vec<FlatTrace> = transaction_traces.into_iter().nth(0).unwrap().into();
		assert_eq!(ordered_traces.len(), 5);
		assert_eq!(ordered_traces[0].parent, None);
		assert_eq!(ordered_traces[0].children, vec![1, 4]);
		assert_eq!(ordered_traces[1].parent, Some(0));
		assert_eq!(ordered_traces[1].children, vec![2, 3]);
		assert_eq!(ordered_traces[2].parent, Some(1));
		assert_eq!(ordered_traces[2].children, vec![]);
		assert_eq!(ordered_traces[3].parent, Some(1));
		assert_eq!(ordered_traces[3].children, vec![]);
		assert_eq!(ordered_traces[4].parent, Some(0));
		assert_eq!(ordered_traces[4].children, vec![]);
	}
}