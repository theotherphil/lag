use either::Either;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use tui::buffer::Buffer;
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, Text, Widget};

fn get_line_offset(line_width: u16, text_area_width: u16, alignment: Alignment) -> u16 {
    match alignment {
        Alignment::Center => (text_area_width / 2).saturating_sub(line_width / 2),
        Alignment::Right => text_area_width.saturating_sub(line_width),
        Alignment::Left => 0,
    }
}

pub struct Gaugagraph<'a, 't, T>
where
    T: Iterator<Item = &'t Text<'t>>,
{
    /// A block to wrap the widget in
    block: Option<Block<'a>>,
    /// Widget style
    style: Style,
    /// The text to display
    text: T,
    /// Should we parse the text for embedded commands
    raw: bool,
    /// Aligenment of the text
    alignment: Alignment,
    ratios: Vec<f64>,
}

impl<'a, 't, T> Gaugagraph<'a, 't, T>
where
    T: Iterator<Item = &'t Text<'t>>,
{
    pub fn new(text: T, ratios: Vec<f64>) -> Gaugagraph<'a, 't, T> {
        Gaugagraph {
            block: None,
            style: Default::default(),
            raw: false,
            text,
            alignment: Alignment::Left,
            ratios: ratios,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Gaugagraph<'a, 't, T> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Gaugagraph<'a, 't, T> {
        self.style = style;
        self
    }

    pub fn raw(mut self, flag: bool) -> Gaugagraph<'a, 't, T> {
        self.raw = flag;
        self
    }

    pub fn alignment(mut self, alignment: Alignment) -> Gaugagraph<'a, 't, T> {
        self.alignment = alignment;
        self
    }
}

impl<'a, 't, 'b, T> Widget for Gaugagraph<'a, 't, T>
where
    T: Iterator<Item = &'t Text<'t>>,
{
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

        self.background(text_area, buf, self.style.bg);

        let style = self.style;
        let mut styled = self.text.by_ref().flat_map(|t| match *t {
            Text::Raw(ref d) => {
                let data: &'t str = d; // coerce to &str
                Either::Left(UnicodeSegmentation::graphemes(data, true).map(|g| Styled(g, style)))
            }
            Text::Styled(ref d, s) => {
                let data: &'t str = d; // coerce to &str
                Either::Right(UnicodeSegmentation::graphemes(data, true).map(move |g| Styled(g, s)))
            }
        });

        let mut line_composer = LineTruncator::new(&mut styled, text_area.width);

        let mut y = 0;
        while let Some((current_line, current_line_width)) = line_composer.next_line() {
            let mut x = get_line_offset(current_line_width, text_area.width, self.alignment);
            for Styled(symbol, style) in current_line {
                // TODO: this, but properly
                // TODO: For now, let's just use a nominal width of 100
                // TODO: we also need to handle bars longer than the underlying line
                if (x as f64) < self.ratios[y as usize] * 100.0 {
                    buf.get_mut(text_area.left() + x, text_area.top() + y)
                        .set_symbol(symbol)
                        .set_style(
                            Style::default()
                                .bg(Color::Rgb(255, 184, 108))
                                .fg(Color::Rgb(40, 42, 54)),
                        );
                } else {
                    buf.get_mut(text_area.left() + x, text_area.top() + y)
                        .set_symbol(symbol)
                        .set_style(*style);
                }

                x += symbol.width() as u16;
            }
            y += 1;
            if y >= text_area.height {
                break;
            }
        }
    }
}

// --- reflow

#[derive(Copy, Clone, Debug)]
pub struct Styled<'a>(pub &'a str, pub Style);

/// A state machine to pack styled symbols into lines.
/// Cannot implement it as Iterator since it yields slices of the internal buffer (need streaming
/// iterators for that).
pub trait LineComposer<'a> {
    fn next_line(&mut self) -> Option<(&[Styled<'a>], u16)>;
}

/// A state machine that truncates overhanging lines.
pub struct LineTruncator<'a, 'b> {
    symbols: &'b mut dyn Iterator<Item = Styled<'a>>,
    max_line_width: u16,
    current_line: Vec<Styled<'a>>,
}

impl<'a, 'b> LineTruncator<'a, 'b> {
    pub fn new(
        symbols: &'b mut dyn Iterator<Item = Styled<'a>>,
        max_line_width: u16,
    ) -> LineTruncator<'a, 'b> {
        LineTruncator {
            symbols,
            max_line_width,
            current_line: vec![],
        }
    }
}

impl<'a, 'b> LineComposer<'a> for LineTruncator<'a, 'b> {
    fn next_line(&mut self) -> Option<(&[Styled<'a>], u16)> {
        if self.max_line_width == 0 {
            return None;
        }

        self.current_line.truncate(0);
        let mut current_line_width = 0;

        let mut skip_rest = false;
        let mut symbols_exhausted = true;
        for Styled(symbol, style) in &mut self.symbols {
            symbols_exhausted = false;

            // Ignore characters wider that the total max width.
            if symbol.width() as u16 > self.max_line_width {
                continue;
            }

            // Break on newline and discard it.
            if symbol == "\n" {
                break;
            }

            if current_line_width + symbol.width() as u16 > self.max_line_width {
                // Exhaust the remainder of the line.
                skip_rest = true;
                break;
            }

            current_line_width += symbol.width() as u16;
            self.current_line.push(Styled(symbol, style));
        }

        if skip_rest {
            for Styled(symbol, _) in &mut self.symbols {
                if symbol == "\n" {
                    break;
                }
            }
        }

        if symbols_exhausted && self.current_line.is_empty() {
            None
        } else {
            Some((&self.current_line[..], current_line_width))
        }
    }
}
