use crate::compiled_class::{CompiledClass, StaticMethodInfo, VirtualMethodInfo};
use crate::stdlib::java_lang_object::JavaObjectRef;
use crate::stdlib::java_lang_string::JavaLangStringRef;
use crate::Isolate;

pub fn new_compiled_class() -> CompiledClass {
    let mut c = CompiledClass::new("java/lang/Char", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@java/lang/Char".to_string(),
        ptr: Some(JavaLangChar::java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.static_methods.push(StaticMethodInfo {
        symbol: "java/lang/Char.init:(C)V".to_string(),
        ptr: Some(JavaLangChar::init as *const u8),
    });
    c.instance_size = std::mem::size_of::<JavaLangChar>() as u32;
    c
}

#[repr(C)]
pub struct JavaLangChar {
    vtable: *const u8,
    value: char,
}

impl JavaLangChar {
    pub unsafe extern "C" fn init(&mut self, v: u32) {
        let c = char::from_u32(v).unwrap();
        self.value = c;
    }

    pub unsafe extern "C" fn java_lang_object_to_string(
        isolate: &mut Isolate,
        ptr: JavaObjectRef,
    ) -> JavaLangStringRef {
        let byte = &*(ptr as *const JavaLangChar);
        let s = byte.value.to_string();
        isolate.new_java_string(&s) as JavaLangStringRef
    }
}
