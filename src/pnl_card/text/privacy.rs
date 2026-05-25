// ---------------------------------------------------------------------------
// Price Privacy
// ---------------------------------------------------------------------------

pub(in crate::pnl_card) fn privacy_price_display(value: &str, obscure: bool) -> String {
    if obscure {
        obscure_price_digits(value)
    } else {
        value.to_string()
    }
}

pub(in crate::pnl_card) fn obscure_price_digits(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return value.to_string();
    }

    let (sign, unsigned) = trimmed
        .strip_prefix('-')
        .map(|value| ("-", value))
        .or_else(|| trimmed.strip_prefix('+').map(|value| ("+", value)))
        .unwrap_or(("", trimmed));
    let (whole, fraction) = unsigned
        .rsplit_once('.')
        .map_or((unsigned, None), |(whole, fraction)| {
            (whole, Some(fraction))
        });
    let whole_digits = whole.chars().filter(|ch| ch.is_ascii_digit()).count();
    let whole_is_zero = whole
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .all(|ch| ch == '0');

    if whole_digits >= 4 {
        return format!("{sign}{}", obscure_last_digits(whole, 2, 'x'));
    }
    if whole_digits >= 2 {
        return format!("{sign}{}", obscure_last_digits(whole, 1, 'x'));
    }
    if whole_digits == 1 && !whole_is_zero {
        return match fraction {
            Some(fraction) if !fraction.is_empty() => {
                format!("{sign}{whole}.{}", "x".repeat(fraction.len().max(2)))
            }
            _ => format!("{sign}x"),
        };
    }

    match fraction {
        Some(fraction) if !fraction.is_empty() => {
            format!("{sign}{whole}.{}", obscure_small_fraction(fraction))
        }
        _ => format!("{sign}x"),
    }
}

fn obscure_small_fraction(fraction: &str) -> String {
    let digit_count = fraction.chars().filter(|ch| ch.is_ascii_digit()).count();
    if digit_count == 0 {
        return fraction.to_string();
    }

    let first_significant_digit = fraction
        .chars()
        .position(|ch| ch.is_ascii_digit() && ch != '0');
    let visible_digits = first_significant_digit
        .map(|idx| idx + 1)
        .unwrap_or_else(|| digit_count.saturating_sub(2))
        .min(digit_count.saturating_sub(2));
    obscure_fraction_after_visible_digits(fraction, visible_digits, 'x')
}

fn obscure_fraction_after_visible_digits(
    value: &str,
    visible_digits: usize,
    mask_char: char,
) -> String {
    let mut seen_digits = 0usize;
    value
        .chars()
        .map(|ch| {
            if !ch.is_ascii_digit() {
                ch
            } else {
                seen_digits += 1;
                if seen_digits > visible_digits {
                    mask_char
                } else {
                    ch
                }
            }
        })
        .collect()
}

fn obscure_last_digits(value: &str, max_digits: usize, mask_char: char) -> String {
    let total_digits = value.chars().filter(|ch| ch.is_ascii_digit()).count();
    let mask_from = total_digits.saturating_sub(max_digits);
    let mut seen_digits = 0usize;

    value
        .chars()
        .map(|ch| {
            if !ch.is_ascii_digit() {
                ch
            } else {
                seen_digits += 1;
                if seen_digits > mask_from {
                    mask_char
                } else {
                    ch
                }
            }
        })
        .collect()
}
