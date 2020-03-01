use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use tui::widgets::{Block, Widget};

pub struct Gaugagraph<'a, 't> {
    /// A block to wrap the widget in
    block: Option<Block<'a>>,
    /// Style for text which does not have a bar behind it
    base_style: Style,
    /// Style for text in front of a bar
    bar_style: Style,
    /// The text to display
    lines: Vec<&'t str>,
    /// The elapsed times for each lines as a proportion of the largest
    /// elapsed time from the log
    ratios: Vec<f64>,
    /// Bar lengths are increased by this factor
    zoom: f64,
}

impl<'a, 't> Gaugagraph<'a, 't> {
    pub fn new(
        lines: Vec<&'t str>,
        base_style: Style,
        bar_style: Style,
        ratios: Vec<f64>,
        zoom: f64,
    ) -> Gaugagraph<'a, 't> {
        Gaugagraph {
            block: None,
            base_style,
            bar_style,
            lines,
            ratios: ratios,
            zoom,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Gaugagraph<'a, 't> {
        self.block = Some(block);
        self
    }
}

impl<'a, 't, 'b> Widget for Gaugagraph<'a, 't> {
    #[inline(never)]
    fn draw(&mut self, area: Rect, buf: &mut Buffer) {
        let text_area = match self.block {
            Some(ref mut b) => {
                b.draw(area, buf);
                b.inner(area)
            }
            None => area,
        };

        if text_area.height < 1 {
            return;
        }

        self.background(text_area, buf, self.base_style.bg);

        let mut y = 0;
        for current_line in self.lines.iter() {
            let bar_end = (self.ratios[y as usize] * text_area.width as f64 * self.zoom) as u16;
            let mut x = 0;

            for symbol in UnicodeSegmentation::graphemes(*current_line, true) {
                if x >= text_area.width {
                    break;
                }

                let style = if x < bar_end {
                    self.bar_style
                } else {
                    self.base_style
                };

                buf.get_mut(text_area.left() + x, text_area.top() + y)
                    .set_symbol(symbol)
                    .set_style(style);

                x += symbol.width() as u16;
            }

            while x < text_area.width.min(bar_end) {
                buf.get_mut(text_area.left() + x, text_area.top() + y)
                    .set_symbol(" ")
                    .set_style(self.bar_style);
                x += 1;
            }

            y += 1;
            if y >= text_area.height {
                break;
            }
        }
    }
}
