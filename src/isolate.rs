use crate::codegen::ClassID;
use crate::stdlib::array::{
    JavaArray, JavaArrayBoolean, JavaArrayBooleanRef, JavaArrayByte, JavaArrayByteRef,
    JavaArrayChar, JavaArrayCharRef, JavaArrayDouble, JavaArrayDoubleRef, JavaArrayFloat,
    JavaArrayFloatRef, JavaArrayInt, JavaArrayIntRef, JavaArrayLong, JavaArrayLongRef,
    JavaArrayRef, JavaArrayShort, JavaArrayShortRef,
};
use crate::stdlib::java_lang_object::{
    java_object_destructor_dummy, JavaObjectDestructor, JavaObjectRef,
};
use crate::stdlib::java_lang_string::{JavaLangString, JavaLangStringRef};
use crate::tracing::Tracer;
use crate::{CodeGen, Stdout};
use std::collections::HashMap;
use std::mem::size_of;
use std::ptr::{null, null_mut};

#[repr(C)]
pub struct Isolate {
    tracer_ptr: *mut Tracer,
    stdout: Box<dyn Stdout>,
    static_objects: Vec<JavaObjectRef>,
    allocated_objects: HashMap<JavaObjectRef, JavaObjectDestructor>,
    class_objects: Vec<ClassObject>,
    java_array_class_id: ClassID,
    bool_java_array_class_id: ClassID,
    byte_java_array_class_id: ClassID,
    char_java_array_class_id: ClassID,
    short_java_array_class_id: ClassID,
    int_java_array_class_id: ClassID,
    long_java_array_class_id: ClassID,
    float_java_array_class_id: ClassID,
    double_java_array_class_id: ClassID,
    java_lang_string_class_id: ClassID,
    class_ids: HashMap<String, ClassID>,
}

type Clinit = extern "C" fn(isolate: &Isolate);

pub extern "C" fn clinit_dummy(_: &Isolate) {}

impl Isolate {
    pub fn new(cc: &CodeGen, stdout: Box<dyn Stdout>) -> Self {
        let tracer_ptr = Box::into_raw(Box::new(Tracer::new()));
        let java_lang_string_class_id = cc.class_id("java/lang/String");
        let java_array_class_id = cc.class_id("Array");
        let bool_java_array_class_id = cc.class_id("ArrayBoolean");
        let byte_java_array_class_id = cc.class_id("ArrayByte");
        let char_java_array_class_id = cc.class_id("ArrayChar");
        let short_java_array_class_id = cc.class_id("ArrayShort");
        let int_java_array_class_id = cc.class_id("ArrayInt");
        let long_java_array_class_id = cc.class_id("ArrayLong");
        let float_java_array_class_id = cc.class_id("ArrayFloat");
        let double_java_array_class_id = cc.class_id("ArrayDouble");

        let class_object_count = cc.max_class_id as usize + 100;
        let mut class_objects = Vec::with_capacity(class_object_count);
        class_objects.resize_with(class_object_count, Default::default);
        Self {
            tracer_ptr,
            static_objects: Vec::new(),
            stdout,
            allocated_objects: HashMap::new(),
            class_objects,
            bool_java_array_class_id,
            byte_java_array_class_id,
            char_java_array_class_id,
            short_java_array_class_id,
            int_java_array_class_id,
            long_java_array_class_id,
            float_java_array_class_id,
            double_java_array_class_id,
            java_lang_string_class_id,
            java_array_class_id,
            // TODO: avoid clone and reuse the same HashMap.
            class_ids: cc.class_ids.clone(),
        }
    }

    pub fn class_id(&self, class_name: &str) -> ClassID {
        self.class_ids[class_name]
    }

    pub fn tracer(&mut self) -> &mut Tracer {
        unsafe { &mut *self.tracer_ptr }
    }

    pub fn stdout(&mut self) -> &mut dyn Stdout {
        &mut *self.stdout
    }

    pub fn stdout_buffer(&self) -> &Vec<u8> {
        self.stdout.buffer()
    }

    pub fn allocate(size: u32) -> JavaObjectRef {
        assert!(size as usize >= size_of::<usize>());
        // Allocate the opaque buffer for the Java object of the size `size`.
        let mut buf: Vec<u8> = Vec::with_capacity(size as usize);
        buf.resize(size as usize, 0);
        // And use the pointer of the buffer as the opaque pointer of the Java object.
        let ptr = buf.as_mut_ptr();
        // Prevent the buffer from being dropped.
        std::mem::forget(buf);
        ptr as JavaObjectRef
    }

    pub extern "C" fn new_java_array(&mut self, length: usize) -> JavaArrayRef {
        let array = Self::new_instance(self, self.java_array_class_id);
        JavaArray::init(array as JavaArrayRef, length, null_mut());
        array as JavaArrayRef
    }

    pub extern "C" fn new_bool_java_array(&mut self, length: usize) -> JavaArrayRef {
        let array = Self::new_instance(self, self.bool_java_array_class_id);
        JavaArrayBoolean::init(array as JavaArrayBooleanRef, length, 0);
        array as JavaArrayRef
    }

    pub extern "C" fn new_byte_java_array(&mut self, length: usize) -> JavaArrayRef {
        let array = Self::new_instance(self, self.byte_java_array_class_id);
        JavaArrayByte::init(array as JavaArrayByteRef, length, 0);
        array as JavaArrayRef
    }

    pub extern "C" fn new_char_java_array(&mut self, length: usize) -> JavaArrayRef {
        let array = Self::new_instance(self, self.char_java_array_class_id);
        JavaArrayChar::init(array as JavaArrayCharRef, length, 0);
        array as JavaArrayRef
    }

    pub extern "C" fn new_short_java_array(&mut self, length: usize) -> JavaArrayRef {
        let array = Self::new_instance(self, self.short_java_array_class_id);
        JavaArrayShort::init(array as JavaArrayShortRef, length, 0);
        array as JavaArrayRef
    }

    pub extern "C" fn new_int_java_array(&mut self, length: usize) -> JavaArrayRef {
        let array = Self::new_instance(self, self.int_java_array_class_id);
        JavaArrayInt::init(array as JavaArrayIntRef, length, 0);
        array as JavaArrayRef
    }

    pub extern "C" fn new_long_java_array(&mut self, length: usize) -> JavaArrayRef {
        let array = Self::new_instance(self, self.long_java_array_class_id);
        JavaArrayLong::init(array as JavaArrayLongRef, length, 0);
        array as JavaArrayRef
    }

    pub extern "C" fn new_float_java_array(&mut self, length: usize) -> JavaArrayRef {
        let array = Self::new_instance(self, self.float_java_array_class_id);
        JavaArrayFloat::init(array as JavaArrayFloatRef, length, 0.0);
        array as JavaArrayRef
    }

    pub extern "C" fn new_double_java_array(&mut self, length: usize) -> JavaArrayRef {
        let array = Self::new_instance(self, self.double_java_array_class_id);
        JavaArrayDouble::init(array as JavaArrayDoubleRef, length, 0.0);
        array as JavaArrayRef
    }

    pub fn new_java_string(&mut self, s: &String) -> JavaObjectRef {
        let string_class_id = self.java_lang_string_class_id;
        let string = Self::new_instance(self, string_class_id);
        JavaLangString::init(string as JavaLangStringRef, s);
        string as JavaObjectRef
    }

    fn add_allocated_object(&mut self, obj: JavaObjectRef, destructor: JavaObjectDestructor) {
        self.allocated_objects.insert(obj, destructor);
    }

    pub extern "C" fn allocate_args(isolate: &mut Isolate, args: &Vec<String>) -> JavaArrayRef {
        let args_array = isolate.new_java_array(args.len());
        for (i, arg) in args.iter().enumerate() {
            let arg = isolate.new_java_string(arg);
            unsafe { (*args_array).set(i as isize, arg as JavaObjectRef) };
        }
        args_array
    }

    #[no_mangle]
    pub extern "C" fn get_class_object(
        isolate: &mut Isolate,
        class_id: ClassID,
        need_initialization: bool,
    ) -> *mut ClassObject {
        if !need_initialization {
            return &mut isolate.class_objects[class_id as usize];
        }
        let class_obj = &isolate.class_objects[class_id as usize];
        if !class_obj.initialized {
            let clinit = class_obj.clinit;
            if clinit as usize != 0 {
                clinit(isolate);
            }
        }
        let class_obj = &mut isolate.class_objects[class_id as usize];
        if !class_obj.initialized {
            class_obj.initialized = true;
        }
        class_obj
    }

    #[no_mangle]
    pub extern "C" fn new_class_object(
        isolate: &mut Isolate,
        class_id: ClassID,
        static_fields_size: u32,
        instance_size: u32,
        vtable: *const u8,
        clinit: Clinit,
    ) -> &mut ClassObject {
        isolate.class_objects[class_id as usize].init(
            static_fields_size,
            instance_size,
            vtable,
            clinit,
        );
        &mut isolate.class_objects[class_id as usize]
    }

    #[no_mangle]
    pub extern "C" fn new_instance(isolate: &mut Isolate, class_id: ClassID) -> JavaObjectRef {
        let class_object = &isolate.class_objects[class_id as usize];
        let obj = Isolate::allocate(class_object.instance_size);
        let vtable_ptr = obj as *mut usize;
        // Set the vtable pointer at the first 8 bytes of the object.
        unsafe {
            std::ptr::write(vtable_ptr, class_object.vtable as usize);
        }
        isolate.add_allocated_object(obj, class_object.destructor);
        obj
    }
}

impl Drop for Isolate {
    fn drop(&mut self) {
        unsafe {
            for (obj, destructor) in &self.allocated_objects {
                destructor(*obj);
            }
            let _ = Box::from_raw(self.tracer_ptr);
        };
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ClassObject {
    static_fields_ptr: *mut u8,
    vtable: *const u8,
    clinit: Clinit,
    initialized: bool,
    instance_size: u32,
    destructor: JavaObjectDestructor,
    // This will only be used by the Rust code.
    opaque: *const u8,
    static_fields: Vec<u8>,
}

impl Default for ClassObject {
    fn default() -> Self {
        Self {
            static_fields_ptr: null_mut(),
            vtable: null(),
            clinit: clinit_dummy,
            initialized: false,
            instance_size: 0,
            destructor: java_object_destructor_dummy,
            opaque: null(),
            static_fields: Vec::new(),
        }
    }
}

impl ClassObject {
    fn init(
        &mut self,
        static_fields_size: u32,
        instance_size: u32,
        vtable: *const u8,
        clinit: Clinit,
    ) {
        self.static_fields = Vec::with_capacity(static_fields_size as usize);
        self.static_fields.resize(static_fields_size as usize, 0);
        self.static_fields_ptr = self.static_fields.as_mut_ptr();
        self.vtable = vtable;
        self.destructor = java_object_destructor_dummy;
        self.initialized = false;
        self.instance_size = instance_size;
        self.clinit = clinit;
        self.opaque = null();
    }

    pub fn set_opaque(&mut self, opaque: *const u8) {
        self.opaque = opaque;
    }

    pub fn set_object(&mut self, offset: usize, obj: JavaObjectRef) {
        // Set the opaque pointer of the Java object to the offset of the class object.
        let ptr = obj as *mut u8;
        unsafe {
            std::ptr::write(
                self.static_fields.as_mut_ptr().add(offset) as *mut *mut u8,
                ptr,
            );
        }
    }
}
