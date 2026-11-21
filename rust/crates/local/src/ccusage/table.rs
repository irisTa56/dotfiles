//! A table of text cells, laid out in aligned columns.

/// A titled table of a header row, body rows and a total row, each of cell texts.
pub(super) struct Table {
    title: String,
    header: Vec<String>,
    body: Vec<Vec<String>>,
    total: Vec<String>,
}

impl Table {
    pub(super) fn new(
        title: String,
        header: Vec<String>,
        body: Vec<Vec<String>>,
        total: Vec<String>,
    ) -> Self {
        Table {
            title,
            header,
            body,
            total,
        }
    }

    /// The table as text, the first column left-aligned and the rest, all numbers,
    /// right-aligned, with a rule under the header and above the total.
    pub(super) fn render(&self) -> String {
        let rows = || {
            std::iter::once(&self.header)
                .chain(&self.body)
                .chain(std::iter::once(&self.total))
        };
        let widths: Vec<usize> = (0..self.header.len())
            .map(|column| {
                rows()
                    .map(|row| row[column].chars().count())
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let rule = widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>()
            .join("  ");
        let line = |row: &[String]| {
            row.iter()
                .zip(&widths)
                .enumerate()
                .map(|(column, (cell, width))| {
                    if column == 0 {
                        format!("{cell:<width$}")
                    } else {
                        format!("{cell:>width$}")
                    }
                })
                .collect::<Vec<_>>()
                .join("  ")
        };
        let mut lines = vec![
            self.title.clone(),
            String::new(),
            line(&self.header),
            rule.clone(),
        ];
        lines.extend(self.body.iter().map(|row| line(row)));
        lines.push(rule);
        lines.push(line(&self.total));
        lines.join("\n")
    }
}
