use crate::compiled_class::{CompiledClass, StaticMethodInfo, VirtualMethodInfo};
use crate::stdlib::java_lang_object::JavaObjectRef;
use crate::stdlib::java_lang_string::JavaLangStringRef;
use crate::Isolate;

pub fn new_compiled_class_java_lang_byte() -> CompiledClass {
    let mut c = CompiledClass::new("java/lang/Byte", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@java/lang/Byte".to_string(),
        ptr: Some(JavaLangByte::java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.static_methods.push(StaticMethodInfo {
        symbol: "java/lang/Byte.init:(B)V".to_string(),
        ptr: Some(JavaLangByte::init as *const u8),
    });
    c.instance_size = std::mem::size_of::<JavaLangByte>() as u32;
    c
}

pub fn new_compiled_class_java_lang_short() -> CompiledClass {
    let mut c = CompiledClass::new("java/lang/Short", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@java/lang/Short".to_string(),
        ptr: Some(JavaLangShort::java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.static_methods.push(StaticMethodInfo {
        symbol: "java/lang/Short.init:(S)V".to_string(),
        ptr: Some(JavaLangShort::init as *const u8),
    });
    c.instance_size = std::mem::size_of::<JavaLangShort>() as u32;
    c
}

pub fn new_compiled_class_java_lang_integer() -> CompiledClass {
    let mut c = CompiledClass::new("java/lang/Integer", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@java/lang/Integer".to_string(),
        ptr: Some(JavaLangInteger::java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.static_methods.push(StaticMethodInfo {
        symbol: "java/lang/Integer.init:(I)V".to_string(),
        ptr: Some(JavaLangInteger::init as *const u8),
    });
    c.instance_size = std::mem::size_of::<JavaLangInteger>() as u32;
    c
}

pub fn new_compiled_class_java_lang_long() -> CompiledClass {
    let mut c = CompiledClass::new("java/lang/Long", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@java/lang/Long".to_string(),
        ptr: Some(JavaLangLong::java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.static_methods.push(StaticMethodInfo {
        symbol: "java/lang/Long.init:(J)V".to_string(),
        ptr: Some(JavaLangLong::init as *const u8),
    });
    c.instance_size = std::mem::size_of::<JavaLangLong>() as u32;
    c
}

pub fn new_compiled_class_java_lang_float() -> CompiledClass {
    let mut c = CompiledClass::new("java/lang/Float", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@java/lang/Float".to_string(),
        ptr: Some(JavaLangFloat::java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.static_methods.push(StaticMethodInfo {
        symbol: "java/lang/Float.init:(F)V".to_string(),
        ptr: Some(JavaLangFloat::init as *const u8),
    });
    c.instance_size = std::mem::size_of::<JavaLangFloat>() as u32;
    c
}

pub fn new_compiled_class_java_lang_double() -> CompiledClass {
    let mut c = CompiledClass::new("java/lang/Double", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/lang/Object.toString:()Ljava/lang/String;@java/lang/Double".to_string(),
        ptr: Some(JavaLangDouble::java_lang_object_to_string as *const u8),
        overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
    });
    c.static_methods.push(StaticMethodInfo {
        symbol: "java/lang/Double.init:(D)V".to_string(),
        ptr: Some(JavaLangDouble::init as *const u8),
    });
    c.instance_size = std::mem::size_of::<JavaLangDouble>() as u32;
    c
}

#[repr(C)]
pub struct JavaLangByte {
    vtable: *const u8,
    value: i8,
}

impl JavaLangByte {
    pub unsafe extern "C" fn init(&mut self, v: i8) {
        self.value = v;
    }

    pub unsafe extern "C" fn java_lang_object_to_string(
        isolate: &mut Isolate,
        ptr: JavaObjectRef,
    ) -> JavaLangStringRef {
        let byte = &*(ptr as *const JavaLangByte);
        let s = byte.value.to_string();
        isolate.new_java_string(&s) as JavaLangStringRef
    }
}

#[repr(C)]
pub struct JavaLangShort {
    vtable: *const u8,
    pub value: i16,
}

impl JavaLangShort {
    pub unsafe extern "C" fn init(&mut self, v: i8) {
        self.value = v as i16;
    }

    pub unsafe extern "C" fn java_lang_object_to_string(
        isolate: &mut Isolate,
        ptr: JavaObjectRef,
    ) -> JavaLangStringRef {
        let short = &*(ptr as *const JavaLangShort);
        let s = short.value.to_string();
        isolate.new_java_string(&s) as JavaLangStringRef
    }
}

#[repr(C)]
pub struct JavaLangInteger {
    vtable: *const u8,
    pub value: i32,
}

impl JavaLangInteger {
    pub unsafe extern "C" fn init(&mut self, v: i8) {
        self.value = v as i32;
    }

    pub unsafe extern "C" fn java_lang_object_to_string(
        isolate: &mut Isolate,
        ptr: JavaObjectRef,
    ) -> JavaLangStringRef {
        let integer = &*(ptr as *const JavaLangInteger);
        let s = integer.value.to_string();
        isolate.new_java_string(&s) as JavaLangStringRef
    }
}

#[repr(C)]
pub struct JavaLangLong {
    vtable: *const u8,
    pub value: i64,
}

impl JavaLangLong {
    pub unsafe extern "C" fn init(&mut self, v: i8) {
        self.value = v as i64;
    }

    pub unsafe extern "C" fn java_lang_object_to_string(
        isolate: &mut Isolate,
        ptr: JavaObjectRef,
    ) -> JavaLangStringRef {
        let long = &*(ptr as *const JavaLangLong);
        let s = long.value.to_string();
        isolate.new_java_string(&s) as JavaLangStringRef
    }
}

#[repr(C)]
pub struct JavaLangFloat {
    vtable: *const u8,
    pub value: f32,
}

impl JavaLangFloat {
    pub unsafe extern "C" fn init(&mut self, v: f32) {
        self.value = v;
    }

    pub unsafe extern "C" fn java_lang_object_to_string(
        isolate: &mut Isolate,
        ptr: JavaObjectRef,
    ) -> JavaLangStringRef {
        let float = &*(ptr as *const JavaLangFloat);
        let s = float.value.to_string();
        isolate.new_java_string(&s) as JavaLangStringRef
    }
}

#[repr(C)]
pub struct JavaLangDouble {
    vtable: *const u8,
    pub value: f64,
}

impl JavaLangDouble {
    pub unsafe extern "C" fn init(&mut self, v: f64) {
        self.value = v;
    }

    pub unsafe extern "C" fn java_lang_object_to_string(
        isolate: &mut Isolate,
        ptr: JavaObjectRef,
    ) -> JavaLangStringRef {
        let double = &*(ptr as *const JavaLangDouble);
        let s = double.value.to_string();
        isolate.new_java_string(&s) as JavaLangStringRef
    }
}
