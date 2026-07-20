mod extract;
mod runtime_syntax;
mod types;
mod util;

use crate::types::ParseBundleResult;
use wasm_bindgen::prelude::*;

/// Parses a Miniblox index-{hash}.js bundle and returns all messages and enums.
#[wasm_bindgen]
pub fn parse(source: &str) -> Result<ParseBundleResult, JsValue> {
    match extract::extract_bundle(source) {
        Ok(result) => Ok(result),
        Err(e) => Err(js_sys::Error::new(&e).into()),
    }
}
