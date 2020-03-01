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

/// The data for a zoomed section of the elapsed time chart.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSection {
    pub points: Vec<(f64, f64)>,
    pub x_bounds: (f64, f64),
    pub y_bounds: (f64, f64),
}

impl ChartState {
    pub fn new(deltas: Vec<f64>) -> ChartState {
        assert!(deltas.len() > 0);
        let mut cumulative_deltas = deltas.clone();
        for i in 1..cumulative_deltas.len() {
            cumulative_deltas[i] += cumulative_deltas[i - 1];
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

    /// If the entire log is visible in the chart then zoom level is 1.0.
    pub fn current_zoom_level(&self) -> f64 {
        self.deltas.len() as f64 / self.interval_length() as f64
    }

    /// Update the current interval if `current_line` is outside it, or close to leaving it.
    pub fn update(&mut self, current_line: usize) {
        // We cannot slide the interval if it already covers the entire log
        if self.interval == (0, self.deltas.len()) {
            return;
        }

        // When the cursor moves within either the left or right margins we will
        // slide the interval if possible
        let margin = self.interval_length() / 8;
        let (lower, upper) = self.interval;

        // Slide right if necessary
        let target_upper = (current_line + margin).min(self.deltas.len());
        if upper < target_upper {
            let offset = target_upper - upper;
            self.interval = (lower + offset, upper + offset);
            return;
        }

        // Slide left if necessary
        let target_lower = if current_line < margin {
            0
        } else {
            current_line - margin
        };
        if target_lower < lower {
            let offset = lower - target_lower;
            self.interval = (lower - offset, upper - offset);
        }
    }

    pub fn interval_length(&self) -> usize {
        self.interval.1 - self.interval.0
    }

    pub fn zoom_in(&mut self, current_line: usize) {
        self.interval = zoom(
            current_line,
            self.deltas.len(),
            self.interval,
            self.horizontal_resolution,
            1.0 / self.zoom_factor,
        );
    }

    pub fn zoom_out(&mut self, current_line: usize) {
        self.interval = zoom(
            current_line,
            self.deltas.len(),
            self.interval,
            self.horizontal_resolution,
            self.zoom_factor,
        );
    }

    pub fn section(&self) -> ChartSection {
        let points: Vec<_> = self
            .cumulative_deltas
            .iter()
            .enumerate()
            .skip(self.interval.0 as usize)
            .step_by(self.interval_length() / self.horizontal_resolution)
            .take(self.horizontal_resolution)
            .map(|(i, d)| (i as f64, *d))
            .collect();

        let first = points[0];
        let last = points[points.len() - 1];

        let x_bounds = (first.0, last.0);
        let y_bounds = (first.1, last.1);

        ChartSection {
            points,
            x_bounds,
            y_bounds,
        }
    }
}

// A `scale_factor` < 1.0 decreases the interval length, i.e. zooms in.
fn zoom(
    current_line: usize,
    num_lines: usize,
    interval: (usize, usize),
    horizontal_resolution: usize,
    scale_factor: f64,
) -> (usize, usize) {
    let current_interval_length = interval.1 - interval.0;
    let target_interval_length = (current_interval_length as f64 * scale_factor) as usize;

    if target_interval_length < horizontal_resolution {
        return interval;
    }

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
        (
            current_line - target_lower_offset,
            current_line + target_upper_offset,
        )
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
                zoom(
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
                zoom_factor: 0.5,
                expected_interval: (0, 199),
            },
            ZoomTestCase {
                description: Some("Above zoom cutoff".into()),
                current_line: 10,
                num_lines: 200,
                interval: (0, 200),
                horizontal_resolution: 100,
                zoom_factor: 0.5,
                expected_interval: (5, 105),
            },
            ZoomTestCase {
                description: None,
                current_line: 280,
                num_lines: 2100,
                interval: (0, 2100),
                horizontal_resolution: 100,
                zoom_factor: 0.5,
                expected_interval: (150, 1140),
            },
        ];
        for test_case in test_cases {
            test_case.run();
        }
    }
}
