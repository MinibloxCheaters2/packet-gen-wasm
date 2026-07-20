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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_extract_bundle() {
		let source = std::fs::read_to_string("../dumpers/bundles/miniblox.js")
			.expect("Failed to read bundle");
		let result = extract::extract_bundle(&source).expect("extract_bundle failed");
		let parsed: serde_json::Value = serde_json::from_str(&result).expect("invalid JSON");

		let msgs = parsed["messages"].as_object().unwrap();
		let enums = parsed["enums"].as_object().unwrap();

		eprintln!("messages: {}", msgs.len());
		eprintln!("enums: {}", enums.len());

		for (name, msg) in msgs.iter().take(5) {
			let fields = msg["fields"].as_array().unwrap();
			eprintln!("  {} ({} fields)", name, fields.len());
			for f in fields.iter().take(3) {
				eprintln!("    no={} name={} kind={}", f["no"], f["name"], f["kind"]);
			}
		}

		for (name, vals) in enums.iter().take(3) {
			eprintln!("  enum {} ({} values)", name, vals.as_array().unwrap().len());
		}

		assert!(!msgs.is_empty(), "should have messages");
		assert!(!enums.is_empty(), "should have enums");
	}
}
