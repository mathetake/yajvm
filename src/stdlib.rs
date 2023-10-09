use crate::CodeGen;

pub mod array;
pub mod java_io;
pub mod java_lang_boolean;
pub mod java_lang_char;
pub mod java_lang_number;
pub mod java_lang_object;
pub mod java_lang_string;
pub mod java_lang_system;

pub fn add_stdlib(cc: &mut CodeGen) {
    cc.add_class(java_lang_object::new_compiled_class());
    cc.add_class(java_lang_string::new_compiled_class());
    cc.add_class(java_lang_system::new_compiled_class());
    cc.add_class(java_lang_boolean::new_compiled_class());
    cc.add_class(java_lang_char::new_compiled_class());
    cc.add_class(java_lang_number::new_compiled_class_java_lang_byte());
    cc.add_class(java_lang_number::new_compiled_class_java_lang_short());
    cc.add_class(java_lang_number::new_compiled_class_java_lang_integer());
    cc.add_class(java_lang_number::new_compiled_class_java_lang_long());
    cc.add_class(java_lang_number::new_compiled_class_java_lang_float());
    cc.add_class(java_lang_number::new_compiled_class_java_lang_double());
    cc.add_class(java_io::new_compiled_class_java_io_print_stream());
    for c in array::new_compiled_classes() {
        cc.add_class(c);
    }
}
