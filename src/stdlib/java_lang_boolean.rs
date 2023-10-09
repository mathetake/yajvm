use crate::compiled_class::{CompiledClass, StaticMethodInfo, VirtualMethodInfo};
use crate::stdlib::java_lang_object::JavaObjectRef;
use crate::stdlib::java_lang_string::JavaLangStringRef;
use crate::Isolate;

pub fn new_compiled_class() -> CompiledClass {
    let mut c = CompiledClass::new("java/lang/Boolean", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@java/lang/Boolean".to_string(),
        ptr: Some(JavaLangBoolean::java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.static_methods.push(StaticMethodInfo {
        symbol: "java/lang/Boolean.init:(Z)V".to_string(),
        ptr: Some(JavaLangBoolean::init as *const u8),
    });
    c.instance_size = std::mem::size_of::<JavaLangBoolean>() as u32;
    c
}

#[repr(C)]
pub struct JavaLangBoolean {
    vtable: *const u8,
    value: bool,
}

impl JavaLangBoolean {
    pub unsafe extern "C" fn init(&mut self, v: u32) {
        self.value = v == 1;
    }

    pub unsafe extern "C" fn java_lang_object_to_string(
        isolate: &mut Isolate,
        ptr: JavaObjectRef,
    ) -> JavaLangStringRef {
        let byte = &*(ptr as *const JavaLangBoolean);
        let s = byte.value.to_string();
        isolate.new_java_string(&s) as JavaLangStringRef
    }
}
