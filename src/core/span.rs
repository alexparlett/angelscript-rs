use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub source_name: Arc<str>,
    pub start: usize,
    pub end: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl Span {
    pub fn new(
        source_name: Arc<str>,
        start: usize,
        end: usize,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self {
            source_name,
            start,
            end,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    pub fn format(&self) -> String {
        format!(
            "{}:{}:{}",
            self.source_name, self.start_line, self.start_column
        )
    }

    pub fn format_range(&self) -> String {
        if self.start_line == self.end_line {
            format!(
                "{}:{}:{}-{}",
                self.source_name, self.start_line, self.start_column, self.end_column
            )
        } else {
            format!(
                "{}:{}:{}-{}:{}",
                self.source_name,
                self.start_line,
                self.start_column,
                self.end_line,
                self.end_column
            )
        }
    }

    pub fn merge(&self, other: &Span) -> Span {
        assert_eq!(self.source_name, other.source_name);

        Span {
            source_name: self.source_name.clone(),
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            start_line: self.start_line.min(other.start_line),
            start_column: if self.start_line == other.start_line {
                self.start_column.min(other.start_column)
            } else if self.start_line < other.start_line {
                self.start_column
            } else {
                other.start_column
            },
            end_line: self.end_line.max(other.end_line),
            end_column: if self.end_line == other.end_line {
                self.end_column.max(other.end_column)
            } else if self.end_line > other.end_line {
                self.end_column
            } else {
                other.end_column
            },
        }
    }
}

#[derive(Clone)]
pub struct SpanBuilder {
    source_name: Arc<str>,
    line_starts: Vec<usize>,
}

impl SpanBuilder {
    pub fn new(source_name: String, content: &str) -> Self {
        let mut line_starts = vec![0];

        for (idx, ch) in content.char_indices() {
            if ch == '\n' {
                line_starts.push(idx + 1);
            }
        }

        Self {
            source_name: Arc::from(source_name.as_str()),
            line_starts,
        }
    }

    pub fn span(&self, start: usize, end: usize) -> Span {
        let (start_line, start_column) = self.line_col(start);
        let (end_line, end_column) = self.line_col(end);

        Span::new(
            self.source_name.clone(),
            start,
            end,
            start_line,
            start_column,
            end_line,
            end_column,
        )
    }

    fn line_col(&self, offset: usize) -> (usize, usize) {
        let line = self
            .line_starts
            .binary_search(&offset)
            .unwrap_or_else(|i| i.saturating_sub(1));

        let line_start = self.line_starts[line];
        let column = offset - line_start;

        (line + 1, column + 1)
    }
}
