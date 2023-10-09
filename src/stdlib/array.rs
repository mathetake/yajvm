use crate::compiled_class::{CompiledClass, VirtualMethodInfo};
use crate::stdlib::java_lang_object::*;
use crate::stdlib::java_lang_string::JavaLangStringRef;
use crate::Isolate;

pub fn new_compiled_classes() -> Vec<CompiledClass> {
    let mut ret = Vec::new();
    let mut c = CompiledClass::new("Array", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@Array".to_string(),
        ptr: Some(java_array_java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.instance_size = std::mem::size_of::<JavaArray>() as u32;
    ret.push(c);

    let mut c = CompiledClass::new("ArrayBoolean", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@ArrayBoolean".to_string(),
        ptr: Some(boolean_java_array_java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.instance_size = std::mem::size_of::<JavaArrayBoolean>() as u32;
    ret.push(c);

    let mut c = CompiledClass::new("ArrayByte", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@ArrayByte".to_string(),
        ptr: Some(byte_java_array_java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.instance_size = std::mem::size_of::<JavaArrayByte>() as u32;
    ret.push(c);

    let mut c = CompiledClass::new("ArrayChar", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@ArrayChar".to_string(),
        ptr: Some(char_java_array_java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.instance_size = std::mem::size_of::<JavaArrayChar>() as u32;
    ret.push(c);

    let mut c = CompiledClass::new("ArrayShort", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@ArrayShort".to_string(),
        ptr: Some(short_java_array_java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.instance_size = std::mem::size_of::<JavaArrayShort>() as u32;
    ret.push(c);

    let mut c = CompiledClass::new("ArrayInt", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@ArrayInt".to_string(),
        ptr: Some(int_java_array_java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.instance_size = std::mem::size_of::<JavaArrayInt>() as u32;
    ret.push(c);

    let mut c = CompiledClass::new("ArrayLong", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@ArrayLong".to_string(),
        ptr: Some(long_java_array_java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.instance_size = std::mem::size_of::<JavaArrayLong>() as u32;
    ret.push(c);

    let mut c = CompiledClass::new("ArrayFloat", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@ArrayFloat".to_string(),
        ptr: Some(float_java_array_java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.instance_size = std::mem::size_of::<JavaArrayFloat>() as u32;
    ret.push(c);

    let mut c = CompiledClass::new("ArrayDouble", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@ArrayDouble".to_string(),
        ptr: Some(double_java_array_java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.instance_size = std::mem::size_of::<JavaArrayDouble>() as u32;
    ret.push(c);

    ret
}

pub type JavaArray = JavaArrayT<JavaObjectRef>;
pub type JavaArrayRef = *mut JavaArray;

pub type JavaArrayBoolean = JavaArrayT<u64>; // For simplicity, we use u64 uniformly.
pub type JavaArrayBooleanRef = *mut JavaArrayBoolean;

pub type JavaArrayByte = JavaArrayT<u64>; // For simplicity, we use u64 uniformly.
pub type JavaArrayByteRef = *mut JavaArrayByte;

pub type JavaArrayChar = JavaArrayT<u64>; // For simplicity, we use u64 uniformly.
pub type JavaArrayCharRef = *mut JavaArrayChar;

pub type JavaArrayShort = JavaArrayT<u64>; // For simplicity, we use u64 uniformly.
pub type JavaArrayShortRef = *mut JavaArrayShort;

pub type JavaArrayInt = JavaArrayT<u64>; // For simplicity, we use u64 uniformly.
pub type JavaArrayIntRef = *mut JavaArrayInt;

pub type JavaArrayLong = JavaArrayT<u64>;
pub type JavaArrayLongRef = *mut JavaArrayLong;

pub type JavaArrayFloat = JavaArrayT<f64>; // For simplicity, we use f64 uniformly.
pub type JavaArrayFloatRef = *mut JavaArrayFloat;

pub type JavaArrayDouble = JavaArrayT<f64>;
pub type JavaArrayDoubleRef = *mut JavaArrayDouble;

/// A struct that represents a Java Array.
#[repr(C)]
pub struct JavaArrayT<T: Copy + Clone> {
    // Even though at the Java user level, there's no method directly on JavaArray,
    // but for implementation convenience, we still put the vtable here. Especially,
    // with the universal way of calling toString, we can simplify the tracing logic because
    // it allows us to call to_string recursively on the objects to get the string representation of
    // arguments.
    vtable: *const u8,
    // pointer to the data of the array. Each element is the pointer to another object.
    pub data: *mut T,
    // the length of the array.
    pub length: usize,
}

impl<T: Copy + Clone> JavaArrayT<T> {
    pub fn init(ptr: *mut JavaArrayT<T>, length: usize, default: T) {
        let data: Vec<T> = {
            let mut data: Vec<T> = Vec::with_capacity(length);
            data.resize(length, default);
            data
        };
        unsafe {
            (*ptr).data = Box::into_raw(data.into_boxed_slice()) as *mut T;
            (*ptr).length = length;
        }
    }

    pub fn get(&self, index: isize) -> T {
        unsafe { *self.data.offset(index) }
    }

    pub fn set(&mut self, index: isize, value: T) {
        unsafe {
            *self.data.offset(index) = value;
        }
    }
}

pub unsafe extern "C" fn java_array_java_lang_object_to_string(
    isolate: &mut Isolate,
    array: JavaArrayRef,
) -> JavaLangStringRef {
    let mut ss = Vec::<String>::new();
    for item in java_array_ref_into_iterator(array) {
        let s = (*to_java_string_ref(isolate, item)).as_str();
        ss.push(s.to_string());
    }
    let joined = format!("[\"{}\"]", ss.join("\", \""));
    isolate.new_java_string(&joined) as JavaLangStringRef
}

pub unsafe extern "C" fn boolean_java_array_java_lang_object_to_string(
    isolate: &mut Isolate,
    array: JavaArrayBooleanRef,
) -> JavaLangStringRef {
    let mut ss = Vec::<String>::new();
    for item in java_array_ref_into_iterator(array) {
        ss.push((item == 1).to_string());
    }
    let joined = format!("[\"{}\"]", ss.join("\", \""));
    isolate.new_java_string(&joined) as JavaLangStringRef
}

pub unsafe extern "C" fn byte_java_array_java_lang_object_to_string(
    isolate: &mut Isolate,
    array: JavaArrayByteRef,
) -> JavaLangStringRef {
    let mut ss = Vec::<String>::new();
    for item in java_array_ref_into_iterator(array) {
        ss.push((item as i8).to_string());
    }
    let joined = format!("[\"{}\"]", ss.join("\", \""));
    isolate.new_java_string(&joined) as JavaLangStringRef
}

pub unsafe extern "C" fn char_java_array_java_lang_object_to_string(
    isolate: &mut Isolate,
    array: JavaArrayCharRef,
) -> JavaLangStringRef {
    let mut ss = Vec::<String>::new();
    for item in java_array_ref_into_iterator(array) {
        let c = char::from_u32(item as u32).unwrap();
        ss.push(c.to_string());
    }
    let joined = format!("[\"{}\"]", ss.join("\", \""));
    isolate.new_java_string(&joined) as JavaLangStringRef
}

pub unsafe extern "C" fn short_java_array_java_lang_object_to_string(
    isolate: &mut Isolate,
    array: JavaArrayShortRef,
) -> JavaLangStringRef {
    let mut ss = Vec::<String>::new();
    for item in java_array_ref_into_iterator(array) {
        ss.push((item as i16).to_string());
    }
    let joined = format!("[\"{}\"]", ss.join("\", \""));
    isolate.new_java_string(&joined) as JavaLangStringRef
}

pub unsafe extern "C" fn int_java_array_java_lang_object_to_string(
    isolate: &mut Isolate,
    array: JavaArrayIntRef,
) -> JavaLangStringRef {
    let mut ss = Vec::<String>::new();
    for item in java_array_ref_into_iterator(array) {
        ss.push((item as i32).to_string());
    }
    let joined = format!("[\"{}\"]", ss.join("\", \""));
    isolate.new_java_string(&joined) as JavaLangStringRef
}

pub unsafe extern "C" fn long_java_array_java_lang_object_to_string(
    isolate: &mut Isolate,
    array: JavaArrayLongRef,
) -> JavaLangStringRef {
    let mut ss = Vec::<String>::new();
    for item in java_array_ref_into_iterator(array) {
        ss.push((item as i64).to_string());
    }
    let joined = format!("[\"{}\"]", ss.join("\", \""));
    isolate.new_java_string(&joined) as JavaLangStringRef
}

pub unsafe extern "C" fn float_java_array_java_lang_object_to_string(
    isolate: &mut Isolate,
    array: JavaArrayFloatRef,
) -> JavaLangStringRef {
    let mut ss = Vec::<String>::new();
    for item in java_array_ref_into_iterator(array) {
        ss.push((item as f32).to_string());
    }
    let joined = format!("[\"{}\"]", ss.join("\", \""));
    isolate.new_java_string(&joined) as JavaLangStringRef
}

pub unsafe extern "C" fn double_java_array_java_lang_object_to_string(
    isolate: &mut Isolate,
    array: JavaArrayDoubleRef,
) -> JavaLangStringRef {
    let mut ss = Vec::<String>::new();
    for item in java_array_ref_into_iterator(array) {
        ss.push((item).to_string());
    }
    let joined = format!("[\"{}\"]", ss.join("\", \""));
    isolate.new_java_string(&joined) as JavaLangStringRef
}

struct JavaArrayIterator<T: Copy + Clone> {
    array: *mut JavaArrayT<T>,
    index: isize,
}

fn java_array_ref_into_iterator<T: Copy + Clone>(
    array: *mut JavaArrayT<T>,
) -> JavaArrayIterator<T> {
    JavaArrayIterator { array, index: 0 }
}

impl<T: Copy + Clone> Iterator for JavaArrayIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let r = unsafe { &*self.array };
        if self.index >= r.length as isize {
            None
        } else {
            let item = r.get(self.index);
            self.index += 1;
            Some(item)
        }
    }
}

impl<T: Copy + Clone> Drop for JavaArrayT<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(
                self.data,
                self.length as usize,
            ));
        }
    }
}
