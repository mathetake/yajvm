use crate::codegen::descriptor::{BaseType, FieldType, MethodType};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::module::Linkage::External;
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicTypeEnum, FloatType, FunctionType, IntType, PointerType, StructType};
use inkwell::values::{BasicValueEnum, FunctionValue, GlobalValue, IntValue, PointerValue};
use inkwell::{AddressSpace, OptimizationLevel};
use std::collections::HashMap;

pub struct CodegenContext<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub execution_engine: ExecutionEngine<'ctx>,
    pub ptr_sized_type: IntType<'ctx>,
    pub void_ptr: PointerType<'ctx>,
    pub java_array_struct_type: StructType<'ctx>,
    pub java_lang_string_struct_type: StructType<'ctx>,
    pub java_lang_char_struct_type: StructType<'ctx>,
    pub java_lang_byte_struct_type: StructType<'ctx>,
    pub java_lang_short_struct_type: StructType<'ctx>,
    pub java_lang_integer_struct_type: StructType<'ctx>,
    pub java_lang_long_struct_type: StructType<'ctx>,
    pub java_lang_float_struct_type: StructType<'ctx>,
    pub java_lang_double_struct_type: StructType<'ctx>,
    pub java_lang_boolean_struct_type: StructType<'ctx>,
    pub bool_type: IntType<'ctx>,
    pub i8_type: IntType<'ctx>,
    pub f64_type: FloatType<'ctx>,
    pub f32_type: FloatType<'ctx>,
    pub i32_type: IntType<'ctx>,
    pub i64_type: IntType<'ctx>,
    pub i16_type: IntType<'ctx>,

    // These are global-replaced at the engine creation time by the corresponding functions
    // of Isolate.
    pub get_class_object_fn: FunctionValue<'ctx>,
    pub new_class_object_fn: FunctionValue<'ctx>,
    pub new_instance_fn: FunctionValue<'ctx>,
    pub new_java_array_fn: FunctionValue<'ctx>,
    pub new_boolean_array_fn: FunctionValue<'ctx>,
    pub new_byte_array_fn: FunctionValue<'ctx>,
    pub new_char_array_fn: FunctionValue<'ctx>,
    pub new_short_array_fn: FunctionValue<'ctx>,
    pub new_int_array_fn: FunctionValue<'ctx>,
    pub new_long_array_fn: FunctionValue<'ctx>,
    pub new_float_array_fn: FunctionValue<'ctx>,
    pub new_double_array_fn: FunctionValue<'ctx>,

    /// holds values corresponding to  the class_id of each class, which will be resolved at the very last phase
    /// of compilation.
    pub class_id_values: HashMap<String, Vec<IntValue<'ctx>>>,
    /// holds values corresponding to the static field offset of each class, which will be resolved at the very last phase
    pub static_field_offset_values: HashMap<String, Vec<IntValue<'ctx>>>,
    /// holds values corresponding to the virtual method offset of each method in a vtable, which will be resolved at the very last phase
    pub virtual_method_offset_values: HashMap<String, Vec<IntValue<'ctx>>>,
}

impl<'ctx> CodegenContext<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        let module = context.create_module("main");
        let builder = context.create_builder();
        let execution_engine = module
            .create_jit_execution_engine(OptimizationLevel::None)
            .unwrap();
        let ptr_sized_type = context.ptr_sized_int_type(execution_engine.get_target_data(), None);
        let void_ptr = context.i8_type().ptr_type(AddressSpace::default());
        let i32_type = context.i32_type();

        // java/lang/String.
        let java_lang_string_struct_type = context.struct_type(
            // vtable, ptr, len: this should match JavaLangString struct.
            &[void_ptr.into(), void_ptr.into(), i32_type.into()],
            false,
        );

        // Java array. (is it named java/lang/Array?)
        let java_array_struct_type = context.struct_type(
            // vtable, ptr, len: this should match JavaLangString struct.
            &[void_ptr.into(), void_ptr.into(), i32_type.into()],
            false,
        );

        // java/lang/byte.
        let java_lang_byte_struct_type = context.struct_type(
            // vtable, i8_type: this should match JavaLangByte struct.
            &[void_ptr.into(), i32_type.into()],
            false,
        );

        // java/lang/short.
        let java_lang_short_struct_type = context.struct_type(
            // vtable, i16_type: this should match JavaLangShort struct.
            &[void_ptr.into(), i32_type.into()],
            false,
        );

        // java/lang/integer.
        let java_lang_integer_struct_type = context.struct_type(
            // vtable, i32_type: this should match JavaLangInteger struct.
            &[void_ptr.into(), i32_type.into()],
            false,
        );

        // java/lang/long.
        let i64_type = context.i64_type();
        let java_lang_long_struct_type = context.struct_type(
            // vtable, i64_type: this should match JavaLangLong struct.
            &[void_ptr.into(), i64_type.into()],
            false,
        );

        // java/lang/float.
        let f32_type = context.f32_type();
        let java_lang_float_struct_type = context.struct_type(
            // vtable, f32_type: this should match JavaLangFloat struct.
            &[void_ptr.into(), f32_type.into()],
            false,
        );

        // java/lang/double.
        let f64_type = context.f64_type();
        let java_lang_double_struct_type = context.struct_type(
            // vtable, f64_type: this should match JavaLangDouble struct.
            &[void_ptr.into(), f64_type.into()],
            false,
        );

        // java/lang/boolean.
        let java_lang_boolean_struct_type = context.struct_type(
            // vtable, bool_type: this should match JavaLangBoolean struct.
            &[void_ptr.into(), i32_type.into()],
            false,
        );

        // java/lang/char.
        let java_lang_char_struct_type = context.struct_type(
            // vtable, i8_type: this should match JavaLangChar struct.
            &[void_ptr.into(), i32_type.into()],
            false,
        );

        let new_class_object_fn = {
            let new_class_object_fn_type = void_ptr.fn_type(
                &[
                    void_ptr.into(), // isolate
                    i32_type.into(), // class_id
                    i32_type.into(), // static_fields_size
                    i32_type.into(), // instance_size
                    void_ptr.into(), // vtable
                    void_ptr.into(), // clinit
                ],
                false,
            );
            module.add_function(
                "__yajvm_new_class_object",
                new_class_object_fn_type,
                Some(External),
            )
        };

        let get_class_object_fn = {
            let get_class_object_fn_type = void_ptr.fn_type(
                &[void_ptr.into(), i32_type.into(), context.bool_type().into()],
                false,
            );
            module.add_function(
                "__yajvm_get_class_object",
                get_class_object_fn_type,
                Some(External),
            )
        };

        let new_instance_fn = {
            let new_instance_fn_type = void_ptr.fn_type(
                &[
                    void_ptr.into(), // isolate
                    i32_type.into(), // class_id
                ],
                false,
            );
            module.add_function("__yajvm_new_instance", new_instance_fn_type, Some(External))
        };

        let new_array_type = void_ptr.fn_type(
            &[
                void_ptr.into(), // isolate
                i32_type.into(), // length
            ],
            false,
        );

        let new_java_array_fn =
            module.add_function("__yajvm_new_java_array", new_array_type, Some(External));
        let new_boolean_array_fn =
            module.add_function("__yajvm_new_boolean_array", new_array_type, Some(External));
        let new_byte_array_fn =
            module.add_function("__yajvm_new_byte_array", new_array_type, Some(External));
        let new_char_array_fn =
            module.add_function("__yajvm_new_char_array", new_array_type, Some(External));
        let new_short_array_fn =
            module.add_function("__yajvm_new_short_array", new_array_type, Some(External));
        let new_int_array_fn =
            module.add_function("__yajvm_new_int_array", new_array_type, Some(External));
        let new_long_array_fn =
            module.add_function("__yajvm_new_long_array", new_array_type, Some(External));
        let new_float_array_fn =
            module.add_function("__yajvm_new_float_array", new_array_type, Some(External));
        let new_double_array_fn =
            module.add_function("__yajvm_new_double_array", new_array_type, Some(External));

        Self {
            context,
            module,
            builder,
            execution_engine,
            ptr_sized_type,
            void_ptr,
            java_array_struct_type,
            java_lang_string_struct_type,
            java_lang_byte_struct_type,
            java_lang_short_struct_type,
            java_lang_integer_struct_type,
            java_lang_long_struct_type,
            java_lang_float_struct_type,
            java_lang_double_struct_type,
            java_lang_boolean_struct_type,
            java_lang_char_struct_type,
            bool_type: context.bool_type(),
            i8_type: context.i8_type(),
            f64_type,
            f32_type,
            i32_type,
            i64_type,
            i16_type: context.i16_type(),
            new_class_object_fn,
            get_class_object_fn,
            new_instance_fn,
            new_java_array_fn,
            new_boolean_array_fn,
            new_byte_array_fn,
            new_char_array_fn,
            new_short_array_fn,
            new_int_array_fn,
            new_long_array_fn,
            new_float_array_fn,
            new_double_array_fn,
            class_id_values: HashMap::default(),
            static_field_offset_values: HashMap::default(),
            virtual_method_offset_values: HashMap::default(),
        }
    }
}

impl<'ctx> CodegenContext<'ctx> {
    pub fn get_or_add_global(&self, symbol: &str, ty: BasicTypeEnum<'ctx>) -> GlobalValue<'ctx> {
        if let Some(g) = self.module.get_global(symbol) {
            g
        } else {
            let global = self.module.add_global(ty, None, symbol);
            global.set_linkage(External);
            global
        }
    }

    // This must match the memory representation of JavaLangString in stdlib/java_lang_string.rs.
    pub fn get_const_string_global(&self, s: &String) -> PointerValue<'ctx> {
        let const_string_symbol = format!("const_string__{}", s);
        return if let Some(g) = self.module.get_global(const_string_symbol.as_str()) {
            g.as_pointer_value()
        } else {
            let const_data = self.context.const_string(s.as_bytes(), false);
            let string_ptr_value = {
                let i8_type = self.context.i8_type();
                let array_type = i8_type.array_type(s.len() as u32);
                let data_symbol = format!("const_string_data__{}", s);
                if let Some(string_ptr) = self.module.get_global(data_symbol.as_str()) {
                    string_ptr.as_pointer_value()
                } else {
                    let string_ptr_value =
                        self.module
                            .add_global(array_type, None, data_symbol.as_str());
                    string_ptr_value.set_initializer(&const_data);
                    string_ptr_value.set_linkage(Linkage::Internal);
                    string_ptr_value
                        .as_pointer_value()
                        .const_cast(self.void_ptr)
                }
            };

            let java_lang_string_vtable = self
                .get_or_add_global(
                    "forward_declared_vtable###java/lang/String",
                    self.void_ptr.array_type(0).into(),
                )
                .as_pointer_value();

            let const_str_obj = {
                let string_object_init_values = {
                    [
                        java_lang_string_vtable.into(),
                        string_ptr_value.into(),
                        self.i32_type.const_int(s.len() as u64, false).into(),
                    ]
                };
                self.context.const_struct(&string_object_init_values, false)
            };

            let global = {
                let global = self.module.add_global(
                    self.java_lang_string_struct_type,
                    None,
                    const_string_symbol.as_str(),
                );
                global.set_initializer(&const_str_obj);
                global.set_linkage(Linkage::Internal);
                global.as_pointer_value()
            };
            global
        };
    }

    /// Compile a method from the Java descriptor.
    pub fn llvm_function_type_from_method_type(
        &self,
        method_type: &MethodType,
        is_static: bool,
    ) -> FunctionType<'ctx> {
        let mut param_types = Vec::new();
        param_types.reserve(method_type.parameter_types.len() + 1);
        // Any function takes RtCtxRef as the first parameter.
        param_types.push(self.void_ptr.into());
        // And if not static, it also takes a self reference
        if !is_static {
            param_types.push(self.void_ptr.into());
        }
        for param_type in &method_type.parameter_types {
            param_types.push(Self::llvm_type_from_field_type(self, param_type));
        }

        let return_type = if let Some(t) = &method_type.return_type {
            // Integers less than i32 will be promoted to i32 as per JVM spec (See 3.11.1):
            // https://docs.oracle.com/javase/specs/jvms/se6/html/Overview.doc.html
            match t {
                FieldType::BaseType(base) => match base {
                    BaseType::Boolean => self.context.bool_type().fn_type(&param_types, false),
                    BaseType::Byte => self.i32_type.fn_type(&param_types, false),
                    BaseType::Char => self.i32_type.fn_type(&param_types, false),
                    BaseType::Double => self.f64_type.fn_type(&param_types, false),
                    BaseType::Float => self.f32_type.fn_type(&param_types, false),
                    BaseType::Int => self.i32_type.fn_type(&param_types, false),
                    BaseType::Long => self.i64_type.fn_type(&param_types, false),
                    BaseType::Short => self.i32_type.fn_type(&param_types, false),
                    BaseType::Void => self.context.void_type().fn_type(&param_types, false),
                },
                FieldType::ObjectType(_) => self.void_ptr.fn_type(&param_types, false),
                FieldType::ArrayType(_) => self.void_ptr.fn_type(&param_types, false),
                FieldType::ObjectTypeJavaLangByte => self.i32_type.fn_type(&param_types, false),
                FieldType::ObjectTypeJavaLangChar => self.i32_type.fn_type(&param_types, false),
                FieldType::ObjectTypeJavaLangDouble => self.f64_type.fn_type(&param_types, false),
                FieldType::ObjectTypeJavaLangFloat => self.f32_type.fn_type(&param_types, false),
                FieldType::ObjectTypeJavaLangInteger => self.i32_type.fn_type(&param_types, false),
                FieldType::ObjectTypeJavaLangLong => self.i64_type.fn_type(&param_types, false),
                FieldType::ObjectTypeJavaLangShort => self.i32_type.fn_type(&param_types, false),
                FieldType::ObjectTypeJavaLangBoolean => self.i32_type.fn_type(&param_types, false),
            }
        } else {
            self.context.void_type().fn_type(&param_types, false)
        };
        return_type
    }

    pub fn llvm_type_from_field_type<
        T: From<IntType<'ctx>>
            + From<IntType<'ctx>>
            + From<FloatType<'ctx>>
            + From<StructType<'ctx>>
            + From<PointerType<'ctx>>,
    >(
        &self,
        field_type: &FieldType,
    ) -> T {
        // Integers less than i32 will be promoted to i32 as per JVM spec (See 3.11.1):
        // https://docs.oracle.com/javase/specs/jvms/se6/html/Overview.doc.html
        match field_type {
            FieldType::BaseType(base) => match base {
                BaseType::Boolean => self.i32_type.into(),
                BaseType::Byte => self.i32_type.into(),
                BaseType::Char => self.i32_type.into(),
                BaseType::Double => self.f64_type.into(),
                BaseType::Float => self.f32_type.into(),
                BaseType::Int => self.i32_type.into(),
                BaseType::Long => self.i64_type.into(),
                BaseType::Short => self.i32_type.into(),
                _ => unreachable!(),
            },
            FieldType::ObjectType(_) => self.void_ptr.into(),
            FieldType::ArrayType(_) => self.void_ptr.into(),
            FieldType::ObjectTypeJavaLangByte => self.i32_type.into(),
            FieldType::ObjectTypeJavaLangChar => self.i32_type.into(),
            FieldType::ObjectTypeJavaLangDouble => self.f64_type.into(),
            FieldType::ObjectTypeJavaLangFloat => self.f32_type.into(),
            FieldType::ObjectTypeJavaLangInteger => self.i32_type.into(),
            FieldType::ObjectTypeJavaLangLong => self.i64_type.into(),
            FieldType::ObjectTypeJavaLangShort => self.i32_type.into(),
            FieldType::ObjectTypeJavaLangBoolean => self.i32_type.into(),
        }
    }

    pub fn get_virtual_method_offset_value(&mut self, method_symbol: &String) -> IntValue<'ctx> {
        let dummy_value = self
            .insert_dummy_value(self.i32_type.into())
            .into_int_value();
        let values = if let Some(values) = self.virtual_method_offset_values.get_mut(method_symbol)
        {
            values
        } else {
            self.virtual_method_offset_values
                .insert(method_symbol.clone(), Vec::default());
            self.virtual_method_offset_values
                .get_mut(method_symbol)
                .unwrap()
        };

        // Insert the dummy value.
        values.push(dummy_value.clone());
        dummy_value // Returned value will be replaced by the real number at the last phase of compilation.
    }

    pub fn get_static_filed_offset_value(
        &mut self,
        class_name: &String,
        field_name: &String,
    ) -> IntValue<'ctx> {
        let dummy_value = self
            .insert_dummy_value(self.i32_type.into())
            .into_int_value();

        let symbol = &format!("{}.{}", class_name, field_name);
        let values = if let Some(val) = self.static_field_offset_values.get_mut(symbol) {
            val
        } else {
            self.static_field_offset_values
                .insert(symbol.clone(), Vec::default());
            self.static_field_offset_values.get_mut(symbol).unwrap()
        };

        values.push(dummy_value);
        dummy_value // Returned value will be replaced by the real number at the last phase of compilation.
    }

    pub fn get_class_id_value(&mut self, class_name: &String) -> IntValue<'ctx> {
        let dummy_value = self
            .insert_dummy_value(self.i32_type.into())
            .into_int_value()
            .clone();

        let values = if let Some(values) = self.class_id_values.get_mut(class_name) {
            values
        } else {
            self.class_id_values
                .insert(class_name.clone(), Vec::default());
            self.class_id_values.get_mut(class_name).unwrap()
        };

        values.push(dummy_value.clone());
        dummy_value // Returned value will be replaced by the real number at the last phase of compilation.
    }

    fn insert_dummy_value(&self, typ: BasicTypeEnum<'ctx>) -> BasicValueEnum<'ctx> {
        let dummy_value = self
            .builder
            .build_load(typ, self.void_ptr.const_null(), "dummy");
        dummy_value
    }
}
