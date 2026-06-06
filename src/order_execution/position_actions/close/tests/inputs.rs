use super::*;

#[test]
fn close_position_inputs_build_reduce_only_side_and_fractional_size() {
    assert_eq!(
        close_position_order_side_and_size("2.5", 0.5),
        Ok((false, "1.25".to_string()))
    );
    assert_eq!(
        close_position_order_side_and_size("-2.5", 1.0),
        Ok((true, "2.5".to_string()))
    );
}

#[test]
fn close_position_inputs_reject_invalid_position_sizes() {
    assert_eq!(
        close_position_order_side_and_size("abc", 0.5),
        Err(ClosePositionInputError::InvalidPositionSize)
    );
    assert_eq!(
        close_position_order_side_and_size("0", 0.5),
        Err(ClosePositionInputError::InvalidPositionSize)
    );
    assert_eq!(
        close_position_order_side_and_size("NaN", 0.5),
        Err(ClosePositionInputError::InvalidPositionSize)
    );
}

#[test]
fn close_position_inputs_reject_invalid_fractions() {
    assert_eq!(
        close_position_order_side_and_size("1", 0.0),
        Err(ClosePositionInputError::InvalidFraction)
    );
    assert_eq!(
        close_position_order_side_and_size("1", 1.25),
        Err(ClosePositionInputError::InvalidFraction)
    );
    assert_eq!(
        close_position_order_side_and_size("1", f64::NAN),
        Err(ClosePositionInputError::InvalidFraction)
    );
}
