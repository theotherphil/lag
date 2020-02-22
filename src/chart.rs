//! Handles logic for scaling and scrolling the chart showing elapsed time against line count

#[derive(Debug, Clone, PartialEq)]
pub struct ChartState {
    /// Elapsed time between log lines as a fraction of total time
    pub deltas: Vec<f64>,
    /// Prefix sum of `deltas`
    pub cumulative_deltas: Vec<f64>,
    /// Inclusive lower and exclusive upper bounds on the lines
    /// included in the currently visible chart region
    pub interval: (usize, usize),
    /// How much to try to multiply interval length by when zooming
    pub zoom_factor: f64,
    /// The number of horizontal "pixels" available to plot the chart
    pub horizontal_resolution: usize,
}

impl ChartState {
    pub fn new(deltas: Vec<f64>) -> ChartState {
        assert!(deltas.len() > 0);
        let mut cumulative_deltas = Vec::with_capacity(deltas.len());
        cumulative_deltas.push(deltas[0]);
        for i in 1..deltas.len() {
            cumulative_deltas.push(cumulative_deltas[i - 1] + deltas[i]);
        }
        let len = deltas.len();
        ChartState {
            deltas,
            cumulative_deltas,
            interval: (0, len),
            zoom_factor: 3.0,
            horizontal_resolution: 100,
        }
    }

    pub fn reset_zoom(&mut self) {
        self.interval = (0, self.deltas.len())
    }

    pub fn interval_length(&self) -> usize {
        self.interval.1 - self.interval.0
    }

    // TODO: implement window scrolling when you get to either end of the visible region
    // TODO: Horizontal zoom has nothing to with vertical deltas. Split them completely?

    pub fn zoom_in(&mut self, current_line: usize) {
        self.interval = zoom_in(
            current_line,
            self.deltas.len(),
            self.interval,
            self.horizontal_resolution,
            self.zoom_factor,
        );
    }

    pub fn zoom_out(&mut self, current_line: usize) {
        self.interval = zoom_out(
            current_line,
            self.deltas.len(),
            self.interval,
            self.horizontal_resolution,
            self.zoom_factor,
        );
    }

    pub fn sample(&self) -> Vec<(f64, f64)> {
        self.cumulative_deltas
            .iter()
            .enumerate()
            .skip(self.interval.0 as usize)
            .step_by(self.interval_length() / self.horizontal_resolution)
            .take(100)
            .map(|(i, d)| {
                (
                    i as f64,
                    *d,
                )
            })
            .collect()
    }
}

fn zoom_in(
    current_line: usize,
    num_lines: usize,
    interval: (usize, usize),
    horizontal_resolution: usize,
    zoom_factor: f64
) -> (usize, usize) {
    let current_interval_length = interval.1 - interval.0;
    let target_interval_length = (current_interval_length as f64 / zoom_factor) as usize;

    if target_interval_length < horizontal_resolution {
        return interval;
    }

    let current_step_size = current_interval_length / horizontal_resolution;
    let lower_offset = (current_line - interval.0) / current_step_size;
    let upper_offset = (interval.1 - current_line) / current_step_size;
    let target_step_size = target_interval_length / horizontal_resolution;

    let lower = current_line - target_step_size * lower_offset;
    let upper = current_line + target_step_size * upper_offset;

    (lower, upper)
}

// TODO: merge with zoom_in
fn zoom_out(
    current_line: usize,
    num_lines: usize,
    interval: (usize, usize),
    horizontal_resolution: usize,
    zoom_factor: f64
) -> (usize, usize) {
    let current_interval_length = interval.1 - interval.0;
    let target_interval_length = (current_interval_length as f64 * zoom_factor) as usize;

    if target_interval_length > num_lines {
        return (0, num_lines);
    }

    let current_step_size = current_interval_length / horizontal_resolution;
    let current_lower_offset = (current_line - interval.0) / current_step_size;
    let current_upper_offset = (interval.1 - current_line) / current_step_size;
    let target_step_size = target_interval_length / horizontal_resolution;

    let target_lower_offset = target_step_size * current_lower_offset;
    let target_upper_offset = target_step_size * current_upper_offset;

    if target_lower_offset > current_line {
        (0, target_interval_length)
    } else if current_line + target_upper_offset > num_lines {
        (num_lines - target_interval_length, num_lines)
    } else {
        (current_line - target_lower_offset, current_line + target_upper_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_state_new() {
        let deltas = vec![0.0, 0.1, 0.4, 0.5];
        let state = ChartState::new(deltas.clone());
        assert_eq!(
            state,
            ChartState {
                deltas,
                cumulative_deltas: vec![0.0, 0.1, 0.5, 1.0],
                interval: (0, 4),
                zoom_factor: 3.0,
                horizontal_resolution: 100,
            }
        );
    }

    #[derive(Debug)]
    struct ZoomTestCase {
        description: Option<String>,
        current_line: usize,
        num_lines: usize,
        interval: (usize, usize),
        horizontal_resolution: usize,
        zoom_factor: f64,
        expected_interval: (usize, usize),
    }

    impl ZoomTestCase {
        fn run(&self) {
            assert_eq!(
                zoom_in(
                    self.current_line,
                    self.num_lines,
                    self.interval,
                    self.horizontal_resolution,
                    self.zoom_factor
                ),
                self.expected_interval,
                "{:?}",
                &self
            );
        }
    }

    #[test]
    fn test_zoom() {
        let test_cases = vec![
            ZoomTestCase {
                description: Some("Below zoom cutoff".into()),
                current_line: 10,
                num_lines: 200,
                interval: (0, 199),
                horizontal_resolution: 100,
                zoom_factor: 2.0,
                expected_interval: (0, 199),
            },
            ZoomTestCase {
                description: Some("Above zoom cutoff".into()),
                current_line: 10,
                num_lines: 200,
                interval: (0, 200),
                horizontal_resolution: 100,
                zoom_factor: 2.0,
                expected_interval: (5, 105),
            },
            ZoomTestCase {
                description: None,
                current_line: 280,
                num_lines: 2100,
                interval: (0, 2100),
                horizontal_resolution: 100,
                zoom_factor: 2.0,
                expected_interval: (150, 1140),
            }
        ];
        for test_case in test_cases {
            test_case.run();
        }
    }
}
