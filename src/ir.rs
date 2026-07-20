use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct FieldInfo {
	pub no: u32,
	pub name: String,
	#[serde(rename = "type")]
	pub field_type: String,
	pub repeated: bool,
	pub oneof: Option<String>,
	pub map: Option<MapInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MapInfo {
	pub key_type: String,
	pub value_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageInfo {
	pub type_name: String,
	pub fields: Vec<FieldInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumInfo {
	pub name: String,
	pub values: Vec<EnumValue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumValue {
	pub no: u32,
	pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleInfo {
	pub messages: Vec<MessageInfo>,
	pub enums: Vec<EnumInfo>,
	#[serde(rename = "cPacketMap")]
	pub c_packet_map: HashMap<String, String>,
	#[serde(rename = "sPacketMap")]
	pub s_packet_map: HashMap<String, String>,
	#[serde(rename = "appendedPackets")]
	pub appended_packets: HashMap<String, String>,
}
