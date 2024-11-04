use super::constants::{INFINITE, SCALE};
use super::types::{Pool, PoolMap, TradePath, Optimizer};
use num_bigint::BigUint;
use num_traits::Zero;

pub fn optimize_amount_out(
    required_trade_paths: Vec<TradePath>,
    pool_map: PoolMap,
    amount_in: BigUint,
) -> (Vec<BigUint>, BigUint) {
    let total_amount = BigUint::from(amount_in.to_string().parse::<u128>().unwrap());
    let mut sorted_required_paths = required_trade_paths.clone();

    sorted_required_paths.sort_by(|a, b| a.tokens.len().cmp(&b.tokens.len()));
    let optimizer = Optimizer::new(sorted_required_paths, pool_map, total_amount.clone());
    let (splits, total_output) = optimizer.optimize();

    (splits, total_output)
}

pub fn optimize_amount_in(
    required_trade_paths: Vec<TradePath>,
    pool_map: PoolMap,
    amount_out: BigUint,
) -> (Vec<BigUint>, BigUint) {
    let total_amount = BigUint::from(amount_out.to_string().parse::<u128>().unwrap());
    let mut sorted_required_paths = required_trade_paths.clone();

    sorted_required_paths.sort_by(|a, b| a.tokens.len().cmp(&b.tokens.len()));
    let optimizer = Optimizer::new(sorted_required_paths, pool_map, total_amount.clone());
    let (splits, total_input) = optimizer.optimize_input();

    (splits, total_input)
}

impl Optimizer {
    fn new(paths: Vec<TradePath>, pools: PoolMap, total_amount: BigUint) -> Self {
        Self {
            paths,
            pools,
            total_amount,
        }
    }

    // Calculate output for a given split
    fn calculate_output(&self, splits: &[f64]) -> f64 {
        let mut total_output = BigUint::zero();

        let active_splits: usize = splits.iter().filter(|&&split| split > 1e-10).count();
        let mut temp_pools = self.pools.clone();
        for (i, &split) in splits.iter().enumerate() {
            if split <= 0.0 {
                continue;
            }

            let split_biguint = Pool::from_f64(split);
            let amount_in = &self.total_amount * &split_biguint / Pool::from_f64(1_f64);

            if amount_in > BigUint::zero() {
                let amount_out = self.paths[i].get_amount_out(&amount_in, &mut temp_pools);
                let hop_count = self.paths[i].tokens.len() - 1;
                // Using a constant hop penalty per hop as simplification
                // This simulates extra gas cost being spent based on route length
                let hop_count_penalty = 1.0 - (0.002 * (hop_count as f64 - 1.0));

                let amount_out =
                    (amount_out * Pool::from_f64(hop_count_penalty)) / Pool::from_f64(1_f64);
                total_output += amount_out;
            }
        }
        // Gas penalty could be a separate module that abstracts the actual logic of finding gas costs based on type of pool
        // This gas penalty is proportional to number of paths across which trade is split 
        let gas_penalty = 1.0 - (0.0001 * (active_splits as f64 - 1.0));
        total_output = (total_output * Pool::from_f64(gas_penalty)) / Pool::from_f64(1_f64);

        Pool::to_f64(&(total_output * BigUint::from(SCALE as u64)))
    }

    fn calculate_max_output(&self) -> Vec<f64> {
        let mut outputs: Vec<f64> = vec![];
        for i in 0..self.paths.len() {
            outputs.push(Pool::to_f64(
                &(self.paths[i].get_max_amount_out(&self.pools) * BigUint::from(SCALE as u64)),
            ));
        }

        outputs
    }

    // Calculate total input tokens required for a given split
    fn calculate_input(&self, splits: &[f64]) -> f64 {
        let mut total_input = BigUint::zero();

        let active_splits: usize = splits.iter().filter(|&&split| split > 1e-10).count();
        let mut temp_pools = self.pools.clone();
        for (i, &split) in splits.iter().enumerate() {
            if split <= 0.0 {
                continue;
            }

            let split_biguint = Pool::from_f64(split);
            let amount_out = &self.total_amount * &split_biguint / Pool::from_f64(1_f64);

            if amount_out > BigUint::zero() {
                let amount_in = self.paths[i].get_amount_in(&amount_out, &mut temp_pools);
                if amount_in.is_none() {
                    return 0.0;
                }
                let amount_in = amount_in.unwrap();
                let hop_count = self.paths[i].tokens.len() - 1;
                // Using a constant hop penalty per hop as simplification
                let hop_count_penalty = 1.0 - (0.002 * (hop_count as f64 - 1.0));

                let amount_in =
                    (amount_in * Pool::from_f64(hop_count_penalty)) / Pool::from_f64(1_f64);
                total_input += amount_in;
            }
        }
        // Gas penalty could be a separate module that abstracts the actual logic of finding gas costs based on type of pool
        let gas_penalty = 1.0 - (0.0001 * (active_splits as f64 - 1.0));
        total_input = (total_input * Pool::from_f64(gas_penalty)) / Pool::from_f64(1_f64);

        1.0 / Pool::to_f64(&(total_input * BigUint::from(SCALE as u64)))
    }

    // Project onto simplex to preserve the constraint that the splis must be >=0 and <=1 and sum to 1
    fn project_onto_simplex(&self, mut splits: Vec<f64>) -> Vec<f64> {
        // First ensure non-negativity
        for split in splits.iter_mut() {
            *split = split.max(0.0);
        }

        // Then normalize to sum to 1
        let sum: f64 = splits.iter().sum();
        if sum > 0.0 {
            for split in splits.iter_mut() {
                *split /= sum;
            }
        } else {
            // If all were zero, reset to equal splits
            let n = splits.len();
            for split in splits.iter_mut() {
                *split = 1.0 / n as f64;
            }
        }

        splits
    }

    fn calculate_gradient(&self, splits: &[f64]) -> Vec<f64> {
        let n = splits.len();
        let mut grad = vec![0.0; n];
        let h = 0.001; // Larger h for numerical stability with big numbers

        // Get base output
        let base_output = self.calculate_output(splits);

        // Calculate gradient for each path
        for i in 0..n {
            let mut splits_plus_h = splits.to_vec();
            // Ensure we maintain sum = 1 while calculating gradient
            splits_plus_h[i] += h;
            // Subtract h/(n-1) from other components to maintain sum = 1
            for j in 0..n {
                if j != i {
                    splits_plus_h[j] -= h / (n - 1) as f64;
                }
            }
            splits_plus_h = self.project_onto_simplex(splits_plus_h);
            let output_plus_h = self.calculate_output(&splits_plus_h);
            grad[i] = (output_plus_h - base_output) / h;
        }

        // Normalize gradient to avoid extremely large steps
        let grad_norm: f64 = grad.iter().map(|x| x * x).sum::<f64>().sqrt();
        if grad_norm > 1e-10 {
            for g in grad.iter_mut() {
                *g /= grad_norm;
            }
        }

        grad
    }

    fn calculate_gradient_input(&self, splits: &[f64]) -> Vec<f64> {
        let n = splits.len();
        let mut grad = vec![0.0; n];
        let h = 0.001; // Larger h for numerical stability with big numbers

        // Get base output
        let base_input = self.calculate_input(splits);

        // Calculate gradient for each path
        for i in 0..n {
            let mut splits_plus_h = splits.to_vec();
            // Ensure we maintain sum = 1 while calculating gradient
            splits_plus_h[i] += h;
            // Subtract h/(n-1) from other components to maintain sum = 1
            for j in 0..n {
                if j != i {
                    splits_plus_h[j] -= h / (n - 1) as f64;
                }
            }
            splits_plus_h = self.project_onto_simplex(splits_plus_h);
            let input_plus_h = self.calculate_input(&splits_plus_h);
            grad[i] = (input_plus_h - base_input) / h;
        }

        // Normalize gradient to avoid extremely large steps
        let grad_norm: f64 = grad.iter().map(|x| x * x).sum::<f64>().sqrt();
        if grad_norm > 1e-10 {
            for g in grad.iter_mut() {
                *g /= grad_norm;
            }
        }

        grad
    }

    // Custom gradient optimization for the situation when input/selling amount is given
    fn optimize(&self) -> (Vec<BigUint>, BigUint) {
        let n_paths = self.paths.len();

        // Start with equal splits
        let mut splits: Vec<f64> = vec![0 as f64; n_paths];
        let mut found_direct_path = false;
        // Start with direct path if available
        for (i, path) in self.paths.iter().enumerate() {
            if path.tokens.len() == 2 {
                splits[i] = 1.0;
                found_direct_path = true;
                break;
            }
        }

        if !found_direct_path {
            splits = vec![1.0 / n_paths as f64; n_paths];
        }
        let mut step_size = 0.1;
        let mut best_splits = splits.clone();
        let mut best_output = self.calculate_output(&splits);

        // consider max iterations as a configurable value
        for _ in 0..250 {
            // Calculate gradient
            let gradient = self.calculate_gradient(&splits);

            // Calculate gradient norm for convergence check
            let gradient_norm: f64 = gradient.iter().map(|g| g * g).sum::<f64>().sqrt();

            // Check convergence
            if gradient_norm < 1e-10 {
                break;
            }

            // Take a step in gradient direction
            let mut new_splits: Vec<f64> = splits
                .iter()
                .zip(gradient.iter())
                .map(|(&s, &g)| s + step_size * g)
                .collect();

            // Project onto simplex
            new_splits = self.project_onto_simplex(new_splits);

            // Calculate new output
            let new_output = self.calculate_output(&new_splits);

            // Update if better
            if new_output > best_output {
                best_output = new_output;
                best_splits = new_splits.clone();
                splits = new_splits;
                step_size *= 1.2; // Increase step size
            } else {
                step_size *= 0.7; // Reduce step size

                if step_size < 1e-10 {
                    break;
                }
            }
        }
        // Convert final results to BigUint
        let mut biguint_splits = Vec::with_capacity(n_paths);
        for &split in &best_splits {
            let split_biguint = Pool::from_f64(split);
            biguint_splits.push(split_biguint);
        }

        // Calculate final output
        let mut temp_pools = self.pools.clone();
        let mut final_output = BigUint::zero();
        for (i, split) in biguint_splits.iter().enumerate() {
            let amount_in = &self.total_amount * split / Pool::from_f64(1_f64);
            let amount_out = self.paths[i].get_amount_out(&amount_in, &mut temp_pools);
            final_output += amount_out;
        }

        (biguint_splits, final_output)
    }

    // Custom gradient descent/ascent optimization for finding optimal input for desired output
    // Keeping separate function for optimizing getting input amounts due to expected significant differences in the algorithm
    fn optimize_input(&self) -> (Vec<BigUint>, BigUint) {
        let n_paths = self.paths.len();

        // We start with the split in proportion to total liquidity along each path
        let max_output = self.calculate_max_output();
        let normalizer: f64 = max_output.iter().sum();
        let mut splits: Vec<f64> = max_output.iter().map(|v| v / normalizer).collect();

        let mut step_size = 0.5;
        let mut best_splits = splits.clone();
        let mut best_input = self.calculate_input(&splits);

        for _ in 0..350 {
            // Calculate gradient
            let gradient = self.calculate_gradient_input(&splits);
            // Calculate gradient norm for convergence check
            let gradient_norm: f64 = gradient.iter().map(|g| g * g).sum::<f64>().sqrt();
            // Check convergence
            if gradient_norm < 1e-16 {
                break;
            }

            // Take a step in gradient direction
            let mut new_splits: Vec<f64> = splits
                .iter()
                .zip(gradient.iter())
                .map(|(&s, &g)| s + step_size * g)
                .collect();

            // Project onto simplex
            new_splits = self.project_onto_simplex(new_splits);

            // Calculate new output
            let new_input = self.calculate_input(&new_splits);

            // Update if better
            if new_input > best_input {
                best_input = new_input;
                best_splits = new_splits.clone();
                splits = new_splits;
                step_size *= 1.5; // Increase step size
            } else {
                step_size *= 0.7; // Reduce step size

                if step_size < 1e-10 {
                    break;
                }
            }
        }
        if best_input == 0.0 {
            return (vec![], INFINITE());
        }
        // Convert final results to BigUint
        let mut biguint_splits = Vec::with_capacity(n_paths);
        for &split in &best_splits {
            let split_biguint = Pool::from_f64(split);
            biguint_splits.push(split_biguint);
        }

        // Calculate final output
        let mut temp_pools = self.pools.clone();
        let mut final_input = BigUint::zero();
        for (i, split) in biguint_splits.iter().enumerate() {
            let amount_out = &self.total_amount * split / Pool::from_f64(1_f64);
            let amount_in = self.paths[i].get_amount_in(&amount_out, &mut temp_pools);

            final_input += amount_in.map_or(INFINITE(), |v| v);
        }

        (biguint_splits, final_input)
    }
}
