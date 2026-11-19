//! Figures as the report shows them.

use std::fmt;

/// A count of tokens, shown with a comma between each group of three digits.
pub(super) struct Count(u64);

impl Count {
    pub(super) fn new(value: u64) -> Self {
        Count(value)
    }
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&group_thousands(&self.0.to_string()))
    }
}

/// `digits` with a comma before each group of three from the right, after a sign
/// where there is one.
fn group_thousands(digits: &str) -> String {
    let (sign, digits) = digits
        .strip_prefix('-')
        .map_or(("", digits), |digits| ("-", digits));
    let mut grouped = String::from(sign);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// A cost in dollars, shown to the cent.
///
/// ccusage reports each cost as a float in dollars. Every figure shown, a cell or a
/// total, is its own sum rounded once here, so each is the nearest cent to what it
/// stands for, and the cells of a row or column need not add up to its total: over many
/// small costs they can be several cents apart. Rounding each cost to cents before
/// summing would make them add up by moving every figure off its own value instead.
pub(super) struct Money(f64);

impl Money {
    pub(super) fn new(value: f64) -> Self {
        Money(value)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a cost in cents is far inside an i64"
        )]
        let cents = (self.0 * 100.0).round() as i64;
        let dollars = group_thousands(&(cents / 100).to_string());
        write!(f, "${dollars}.{:02}", cents % 100)
    }
}

/// A share, shown as a percentage to one decimal place.
pub(super) struct Percent(f64);

impl Percent {
    pub(super) fn new(value: f64) -> Self {
        Percent(value)
    }
}

impl fmt::Display for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a share in tenths of a percent is at most 1000"
        )]
        let permille = (self.0 * 1000.0).round() as i64;
        write!(f, "{}.{}%", permille / 10, permille % 10)
    }
}
