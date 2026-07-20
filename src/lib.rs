mod extract;
mod util;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_bundle(source: &str) -> String {
	match extract::extract_bundle(source) {
		Ok(json) => json,
		Err(e) => serde_json::json!({"error": e}).to_string(),
	}
}
