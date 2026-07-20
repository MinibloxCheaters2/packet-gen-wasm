use wasm_bindgen::prelude::*;

#[wasm_bindgen(getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct MappedField {
    pub no: f64,
    pub name: String,
    pub kind: String,
    #[wasm_bindgen(js_name = "T")]
    pub t: Option<String>,
    pub repeated: bool,
    pub opt: bool,
    pub oneof: Option<String>,
    pub map: Option<MappedMap>,
}

#[wasm_bindgen(getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct MappedMap {
    #[wasm_bindgen(js_name = "K")]
    pub k: Option<String>,
    #[wasm_bindgen(js_name = "V")]
    pub v: Option<String>,
}

#[wasm_bindgen(getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct MappedMessage {
    #[wasm_bindgen(js_name = "typeName")]
    pub type_name: String,
    pub fields: Vec<MappedField>,
}

#[wasm_bindgen(getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct MappedEnumEntry {
    pub no: f64,
    pub name: String,
}

#[wasm_bindgen(getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct ParseBundleResult {
    pub messages: Vec<MappedMessage>,
    pub enums: Vec<MappedEnumGroup>,
}

#[wasm_bindgen(getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct MappedEnumGroup {
    pub name: String,
    pub values: Vec<MappedEnumEntry>,
}
