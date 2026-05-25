use super::response::{
    PERP_DELTAS_ENTRY_LIMIT, PERP_DELTAS_RESPONSE_MAX_BYTES, append_perp_deltas_response_chunk,
    parse_perp_deltas_response, parse_ticker_positions_response,
};

mod perp_deltas;
mod ticker_positions;

fn ticker_positions_or_panic(text: &str) -> crate::hyperdash_api::models::TickerPositions {
    match parse_ticker_positions_response(text) {
        Ok(parsed) => parsed,
        Err(error) => panic!("positioning response should parse: {error}"),
    }
}

fn ticker_positions_error_or_panic(text: &str) -> String {
    match parse_ticker_positions_response(text) {
        Ok(_) => panic!("graphql error should be surfaced"),
        Err(error) => error,
    }
}

fn perp_deltas_or_panic(text: &str) -> crate::hyperdash_api::models::PerpDeltas {
    match parse_perp_deltas_response(text) {
        Ok(parsed) => parsed,
        Err(error) => panic!("perp deltas response should parse: {error}"),
    }
}

fn perp_deltas_error_or_panic(text: &str) -> String {
    match parse_perp_deltas_response(text) {
        Ok(_) => panic!("graphql error should be surfaced"),
        Err(error) => error,
    }
}

fn chunk_error_or_panic(body: &mut Vec<u8>, chunk: &[u8]) -> String {
    match append_perp_deltas_response_chunk(body, chunk) {
        Ok(()) => panic!("oversized response body should be rejected"),
        Err(error) => error,
    }
}

fn append_chunk_or_panic(body: &mut Vec<u8>, chunk: &[u8]) {
    if let Err(error) = append_perp_deltas_response_chunk(body, chunk) {
        panic!("response body at exact cap should be accepted: {error}");
    }
}
