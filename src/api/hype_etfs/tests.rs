use super::*;

mod bhyp;
mod thyp;

fn thyp_response_or_panic(json: &str) -> ThypResponse {
    match serde_json::from_str(json) {
        Ok(response) => response,
        Err(error) => panic!("valid THYP fixture: {error}"),
    }
}

fn bhyp_response_or_panic(json: &str) -> BhypResponse {
    match serde_json::from_str(json) {
        Ok(response) => response,
        Err(error) => panic!("valid BHYP fixture: {error}"),
    }
}

fn f64_or_panic(value: Option<f64>, label: &str) -> f64 {
    match value {
        Some(value) => value,
        None => panic!("missing {label}"),
    }
}
