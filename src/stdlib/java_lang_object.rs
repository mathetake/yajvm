use crate::compiled_class::{CompiledClass, VirtualMethodInfo};
use crate::stdlib::java_lang_string::JavaLangStringRef;
use crate::Isolate;
use std::ptr::null_mut;

pub fn new_compiled_class() -> CompiledClass {
    let mut c = CompiledClass::new("java/lang/Object", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;".to_string(),
        ptr: Some(java_lang_object_to_string as *const u8),
        overrides: None,
    });
    c.instance_size = 0;
    c
}

pub unsafe extern "C" fn java_lang_object_to_string(
    _isolate: &mut Isolate,
    _ptr: JavaObjectRef,
) -> JavaLangStringRef {
    null_mut()
}

/// The type of a Java object reference.
pub type JavaObjectRef = *mut u8;

/// Represents a destructor of a Java object.
pub type JavaObjectDestructor = fn(JavaObjectRef);

pub fn java_object_destructor_dummy(_: JavaObjectRef) {}

pub fn to_java_string_ref(ctx: &mut Isolate, obj_ref: JavaObjectRef) -> JavaLangStringRef {
    unsafe {
        // toString always the first method in the vtable.
        // To get the first method, we can simply dereference the pointer twice:
        // 1. Dereference the pointer to get the vtable.
        // 2. Dereference the vtable to get the first method.
        let vtable = *(obj_ref as *mut *const u8);
        let f = *(vtable as *const extern "C" fn(&mut Isolate, JavaObjectRef) -> JavaLangStringRef);
        f(ctx, obj_ref)
    }
}
