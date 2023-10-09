use crate::compiled_class::{CompiledClass, VirtualMethodInfo};
use crate::stdlib::java_lang_object::*;
use crate::Isolate;

pub fn new_compiled_class() -> CompiledClass {
    let mut c = CompiledClass::new("java/lang/String", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@java/lang/String".to_string(),
        ptr: Some(JavaLangString::java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.instance_size = std::mem::size_of::<JavaLangString>() as u32;
    c
}

#[repr(C)]
/// Corresponds to Ljava/lang/String.
pub struct JavaLangString {
    vtable: *const u8,
    pub ptr: *const u8,
    pub len: usize,
}

pub type JavaLangStringRef = *mut JavaLangString;

impl JavaLangString {
    pub fn init(ptr: JavaLangStringRef, s: &String) {
        let len = s.len();
        let data_ptr = Box::into_raw(s.clone().into_boxed_str()) as *mut u8;
        unsafe {
            (*ptr).ptr = data_ptr;
            (*ptr).len = len;
        }
    }

    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn destructor(obj: JavaObjectRef) {
        unsafe {
            let _ = Box::from_raw(obj as *mut Self);
        }
    }

    pub unsafe extern "C" fn java_lang_object_to_string(
        _: &mut Isolate,
        ptr: JavaObjectRef,
    ) -> JavaLangStringRef {
        // TODO: should I allocate a new JavaLangString here? Not sure the actual semantics of Java.
        ptr as JavaLangStringRef
    }
}

impl Drop for JavaLangString {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(
                self.ptr as *mut u8,
                self.len,
            ));
        }
    }
}
