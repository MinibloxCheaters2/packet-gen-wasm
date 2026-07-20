mod extract;
mod types;
mod util;

use crate::types::ParseBundleResult;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_bundle(source: &str) -> Result<ParseBundleResult, JsValue> {
	match extract::extract_bundle(source) {
		Ok(result) => Ok(result),
		Err(e) => Err(js_sys::Error::new(&e).into()),
	}
}
