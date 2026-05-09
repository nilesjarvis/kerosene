use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Timeframe {
    M1, H1
}

impl Timeframe {
    fn label(self) -> &'static str {
        match self {
            Timeframe::M1 => "1m",
            Timeframe::H1 => "1H",
        }
    }
}

impl Display for Timeframe {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.label())
    }
}

fn main() {
    let t = Timeframe::M1;
    println!("{}", t);
}
