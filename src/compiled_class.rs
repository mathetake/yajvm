use crate::Isolate;
use std::string::ToString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledClass {
    pub class_name: String,
    pub static_fields: Vec<String>,
    pub static_methods: Vec<StaticMethodInfo>,
    pub virtual_methods: Vec<VirtualMethodInfo>,
    pub instance_size: u32,
    pub clinit: Option<extern "C" fn(_isolate: &mut Isolate)>,
    pub opaque: Vec<u8>,
    pub super_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualMethodInfo {
    pub symbol: String,
    pub ptr: Option<*const u8>,
    pub overrides: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticMethodInfo {
    pub symbol: String,
    pub ptr: Option<*const u8>,
}

impl CompiledClass {
    pub fn new(class_name: &str, super_class: Option<String>) -> Self {
        Self {
            class_name: class_name.to_string(),
            static_fields: Default::default(),
            static_methods: Default::default(),
            virtual_methods: Default::default(),
            instance_size: 0,
            clinit: None,
            opaque: Default::default(),
            super_class,
        }
    }

    pub fn static_field_size(&self) -> u32 {
        (self.static_fields.len() * 8) as u32
    }
}
