mod codegen_class;
mod codegen_class_static_fields;
mod codegen_context;
pub mod descriptor;

pub use codegen_class::*;
pub use codegen_context::*;
use std::collections::{HashMap, HashSet};

use crate::compiled_class::CompiledClass;
use crate::Isolate;
use inkwell::context::Context;
use inkwell::values::{BasicValue, FunctionValue, PointerValue};

pub struct CodeGen<'ctx> {
    ctx: *mut Context,
    pub cc: CodegenContext<'ctx>,
    tracing_enabled: bool,
    pub class_ids: HashMap<String, ClassID>,
    class_parents: Vec<ClassID>,
    java_lang_object_class_id: ClassID,
    pub max_class_id: ClassID,

    pub classes: Vec<CompiledClass>,
    main_class_symbol: String,
    vtable_offsets: HashMap<String, usize>, // method symbol -> offset in vtable.
    vtables: HashMap<ClassID, Vec<PointerValue<'ctx>>>,
}

pub type ClassID = u32;

impl<'ctx> CodeGen<'ctx> {
    pub fn new(main_class_name: &str) -> Self {
        let ctx = Box::into_raw(Box::new(Context::create()));
        let cc = CodegenContext::new(unsafe { &*ctx });
        let main_class_symbol = format!("{}.main:([Ljava/lang/String;)V", main_class_name);

        Self {
            ctx,
            cc,
            class_parents: Vec::new(),
            tracing_enabled: false,
            classes: Vec::new(),
            class_ids: HashMap::new(),
            max_class_id: 0,
            java_lang_object_class_id: 0,
            main_class_symbol,
            vtable_offsets: HashMap::new(),
            vtables: HashMap::new(),
        }
    }

    pub fn class_id(&self, class_name: &str) -> ClassID {
        *self.class_ids.get(class_name).unwrap()
    }

    pub fn compile(&mut self, path: &str) {
        let mut compiler = ClassFileCompiler::new(String::from(path), self.tracing_enabled);
        compiler.initialize_class_object_info();
        compiler.compile_methods(&mut self.cc);
        let class = compiler.as_class();
        self.classes.push(class);
    }

    pub fn enable_tracing(&mut self) {
        self.tracing_enabled = true;
    }

    pub fn dump_llvm_module(&mut self, path: &str) {
        self.cc.module.print_to_file(path).unwrap();
    }

    pub fn add_class(&mut self, class: CompiledClass) {
        self.classes.push(class);
        let class = self.classes.last_mut().unwrap();

        if let Some(clinit) = class.clinit {
            // This case is a Rust library.
            let clinit_fn = self.cc.module.add_function(
                format!("{}.clinit", class.class_name).as_str(),
                self.cc
                    .context
                    .void_type()
                    // Takes Isolate as the first argument.
                    .fn_type(&[self.cc.void_ptr.into()], false),
                None,
            );
            self.cc
                .execution_engine
                .add_global_mapping(&clinit_fn, clinit as usize);
        }

        for static_method in &class.static_methods {
            if let Some(ptr) = static_method.ptr {
                let desc = static_method
                    .symbol
                    .chars()
                    .skip_while(|c| *c != ':')
                    .skip(1)
                    .collect::<String>();

                let method_type = descriptor::parse_method_descriptor(&desc);
                let func_type = self
                    .cc
                    .llvm_function_type_from_method_type(&method_type, true);
                let func =
                    self.cc
                        .module
                        .add_function(static_method.symbol.as_str(), func_type, None);
                self.cc
                    .execution_engine
                    .add_global_mapping(&func, ptr as usize);
            }
        }
    }
}

impl Drop for CodeGen<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = *self.ctx;
        };
    }
}

impl<'ctx> CodeGen<'ctx> {
    pub fn done_compilation(&mut self) {
        self.assign_class_ids();
        self.resolve_static_field_offsets();
        self.build_inheritance_tree();
        self.construct_vtables();
        self.compile_main_function();
    }

    fn build_inheritance_tree(&mut self) {
        assert!(
            !self.class_ids.is_empty(),
            "class_ids must be assigned before building inheritance tree"
        );

        self.class_parents
            .resize((self.max_class_id + 1) as usize, 0);

        // The root class is java/lang/Object.
        let java_lang_object_class_id = *self.class_ids.get("java/lang/Object").unwrap();
        self.java_lang_object_class_id = java_lang_object_class_id;

        for class in &self.classes {
            let child = *self.class_ids.get(&class.class_name).unwrap();
            let parent = if let Some(parent) = &class.super_class {
                *self.class_ids.get(parent).unwrap()
            } else {
                java_lang_object_class_id
            };
            self.class_parents[child as usize] = parent;
        }
    }

    pub fn assign_class_ids(&mut self) {
        // Sort the classes by name to make the order of class IDs deterministic.
        self.classes.sort_by(|a, b| a.class_name.cmp(&b.class_name));
        for (i, class) in self.classes.iter().enumerate() {
            self.class_ids
                .insert(class.class_name.clone(), i as ClassID);
            self.max_class_id = i as ClassID;
        }

        for (class_name, id) in self.class_ids.iter() {
            let class_id = self.cc.i32_type.const_int(*id as u64, false);
            if let Some(vals) = self.cc.class_id_values.get(class_name) {
                for val in vals {
                    val.replace_all_uses_with(class_id.into());
                    // Removes the dummy load.
                    val.as_instruction_value().unwrap().erase_from_basic_block()
                }
            }
        }
    }

    fn resolve_static_field_offsets(&mut self) {
        for i in 0..self.classes.len() {
            let class = &mut self.classes[i];
            class.static_fields.sort_by(|a, b| a.cmp(&b));

            for (j, field) in class.static_fields.iter().enumerate() {
                let offset = j * 8; // Each static field takes 8 bytes.
                let offset_symbol = format!("{}.{}", class.class_name, field);
                let resolved = self.cc.i32_type.const_int(offset as u64, false);
                if let Some(vals) = self.cc.static_field_offset_values.get(&offset_symbol) {
                    for val in vals {
                        val.replace_all_uses_with(resolved.into());
                        // Removes the dummy load.
                        val.as_instruction_value().unwrap().erase_from_basic_block();
                    }
                }
            }
        }
    }

    fn declare_main_function(&self) -> FunctionValue<'ctx> {
        self.cc.module.add_function(
            "main",
            self.cc
                .context
                .void_type()
                // (Isolate, args)
                .fn_type(&[self.cc.void_ptr.into(), self.cc.void_ptr.into()], false),
            None,
        )
    }

    fn compile_main_function(&mut self) {
        let main = self.declare_main_function();
        let entry = self.cc.context.append_basic_block(main, "entry");
        self.cc.builder.position_at_end(entry);

        let isolate_ptr = main.get_nth_param(0).unwrap().into_pointer_value();

        // 1. Class object allocations.
        for class in &self.classes {
            let clinit = if let Some(clinit) = self
                .cc
                .module
                .get_function(format!("{}.clinit", class.class_name).as_str())
            {
                clinit.as_global_value().as_pointer_value()
            } else {
                self.cc.void_ptr.const_null()
            };
            self.compile_class_object_allocation(
                isolate_ptr,
                &class.class_name,
                class.static_field_size(),
                class.instance_size,
                clinit,
            )
        }

        // Call initialization.
        let need_initialization_true = self.cc.context.bool_type().const_int(1, false);
        for i in 0..=self.max_class_id {
            let class_id = self.cc.i32_type.const_int(i as u64, false);
            self.cc.builder.build_call(
                self.cc.get_class_object_fn,
                &[
                    isolate_ptr.into(),
                    class_id.into(),
                    need_initialization_true.into(),
                ],
                "get_class_object",
            );
        }

        // After the class object initialization, we can create the array object for args.
        let allocate_args = {
            let func = self.cc.module.add_function(
                "allocate_args",
                self.cc
                    .void_ptr
                    // (Isolate, Args)
                    .fn_type(&[self.cc.void_ptr.into(), self.cc.void_ptr.into()], false),
                None,
            );
            self.cc
                .execution_engine
                .add_global_mapping(&func, Isolate::allocate_args as usize);
            func
        };
        let args = main.get_nth_param(1).unwrap().into_pointer_value();
        let args_allocated = self
            .cc
            .builder
            .build_call(
                allocate_args,
                &[isolate_ptr.into(), args.into()],
                "call main function",
            )
            .try_as_basic_value()
            .left()
            .unwrap();

        // Finally, call the main function.
        let main_fn = self
            .cc
            .module
            .get_function(self.main_class_symbol.as_str())
            .unwrap();
        self.cc.builder.build_call(
            main_fn,
            &[isolate_ptr.into(), args_allocated.into()],
            "call main function",
        );
        self.cc.builder.build_return(None);
    }

    fn compile_class_object_allocation(
        &self,
        isolate_ptr: PointerValue,
        class_name: &str,
        static_field_size: u32,
        instance_size: u32,
        clinit: PointerValue,
    ) {
        let class_id = self
            .cc
            .context
            .i32_type()
            .const_int(self.class_id(class_name) as u64, false);
        let static_field_size = self
            .cc
            .context
            .i32_type()
            .const_int(static_field_size as u64, false);
        let instance_size = self
            .cc
            .context
            .i32_type()
            .const_int(instance_size as u64, false);

        let vtable = {
            let vtable_symbol = format!("vtable###{}", class_name);
            self.cc
                .module
                .get_global(vtable_symbol.as_str())
                .unwrap()
                .as_pointer_value()
        };

        self.cc.builder.build_call(
            self.cc.new_class_object_fn,
            &[
                isolate_ptr.into(),
                class_id.into(),
                static_field_size.into(),
                instance_size.into(),
                vtable.into(),
                clinit.into(),
            ],
            "new_class_object",
        );
    }

    fn construct_vtables(&mut self) {
        assert!(
            !self.class_parents.is_empty(),
            "class_parents must be built before constructing vtables"
        );

        let mut done = HashSet::new();
        for i in 0..self.classes.len() {
            let i = i as ClassID;
            if !done.contains(&i) {
                self.construct_vtable(i, &mut done);
                done.insert(i);
            }
        }
    }

    fn construct_vtable(&mut self, i: ClassID, done: &mut HashSet<ClassID>) {
        let is_java_lang_object = i == self.java_lang_object_class_id;

        // Recursively build the parent vtable first.
        let parent_class_id = *self.class_parents.get(i as usize).unwrap();
        if !done.contains(&parent_class_id) && !is_java_lang_object {
            self.construct_vtable(parent_class_id, done);
            done.insert(parent_class_id);
        }
        let class = &mut self.classes.get_mut(i as usize).unwrap();

        // Clone the parent vtable.
        let mut methods: Vec<PointerValue> = if is_java_lang_object {
            Vec::default()
        } else {
            self.vtables.get(&parent_class_id).unwrap().clone()
        };

        // Do the in-place sort the class.virtual_methods by its name to make the order of vtable deterministic.
        class
            .virtual_methods
            .sort_by(|a, b| a.symbol.cmp(&b.symbol));

        for method in class.virtual_methods.iter() {
            let symbol = &method.symbol;

            let func_ptr = if let Some(method_ptr) = method.ptr {
                // Find the descriptor of the method == between ':' and '@' of the method:
                // e.g. "java/lang/Object.main:([Ljava/lang/String;)V@my/org/MyClass" -> "([Ljava/lang/String;)V"
                let desc = symbol
                    .chars()
                    .skip_while(|c| *c != ':')
                    .skip(1)
                    .take_while(|c| *c != '@')
                    .collect::<String>();
                let method_type = descriptor::parse_method_descriptor(&desc);

                let f = self.cc.module.add_function(
                    symbol.as_str(),
                    self.cc
                        .llvm_function_type_from_method_type(&method_type, false),
                    None,
                );
                self.cc
                    .execution_engine
                    .add_global_mapping(&f, method_ptr as usize);
                f.as_global_value().as_pointer_value()
            } else {
                let func = self.cc.module.get_function(symbol.as_str()).unwrap();
                func.as_global_value().as_pointer_value()
            };

            if let Some(overrides) = &method.overrides {
                let offset = self.vtable_offsets.get(overrides).unwrap();
                // This method is already in the vtable. so replace it with the new one.
                methods[*offset] = func_ptr;
            } else {
                let offset = methods.len();
                methods.push(func_ptr);
                self.vtable_offsets.insert(symbol.clone(), offset);

                if let Some(vals) = self.cc.virtual_method_offset_values.get(symbol) {
                    for val in vals {
                        let offset = self.cc.i32_type.const_int(offset as u64, false);
                        val.replace_all_uses_with(offset.into());
                        // Removes the dummy load.
                        val.as_instruction_value().unwrap().erase_from_basic_block();
                    }
                }
            }
        }

        let vtable_ptr = {
            let symbol = format!("vtable###{}", class.class_name);
            let vtable = self.cc.module.add_global(
                self.cc.void_ptr.array_type(methods.len() as u32),
                None,
                symbol.as_str(),
            );

            let forward_declared_symbol = format!("forward_declared_{symbol}");
            if let Some(forwarded_declaration) =
                self.cc.module.get_global(forward_declared_symbol.as_str())
            {
                forwarded_declaration
                    .as_pointer_value()
                    .replace_all_uses_with(vtable.as_pointer_value());
                unsafe { forwarded_declaration.delete() }
            }
            vtable
        };
        vtable_ptr.set_initializer(&self.cc.void_ptr.const_array(&methods[..]));
        self.vtables.insert(i, methods);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::compiled_class::{StaticMethodInfo, VirtualMethodInfo};
    use crate::stdlib::array::JavaArrayRef;
    use crate::Isolate;
    use inkwell::execution_engine::JitFunction;
    use inkwell::values::BasicValue;
    use std::mem::size_of;
    use std::ptr::null_mut;

    #[test]
    fn test_done_compilation() {
        let mut codegen = CodeGen::new("Main");
        let mut main = CompiledClass::new("Main", None);
        let isolate: [u64; 6] = [0, 0, 0, 0, 0, 0];
        extern "C" fn main_fn(isolate: &mut Isolate, _args: *mut u8) {
            unsafe {
                let ptr = std::mem::transmute::<_, *mut u64>(isolate);
                (*ptr) = 0xdeadbeaf;
            }
        }
        extern "C" fn new_class_object(
            isolate: &mut Isolate,
            class_id: ClassID,
            _static_fields_size: u32,
            _instance_size: u32,
            _vtable: *const u8,
            _clinit: *const u8,
        ) {
            match class_id {
                0 => unsafe {
                    let ptr = std::mem::transmute::<_, *mut u64>(isolate);
                    (*ptr.offset(1)) = 0xdeadbeaf_beafdead;
                },
                1 => unsafe {
                    let ptr = std::mem::transmute::<_, *mut u64>(isolate);
                    (*ptr.offset(2)) = 0xdeadbeaf_beafdead;
                },
                _ => {}
            }
        }
        extern "C" fn get_class_object(
            isolate: &mut Isolate,
            class_id: ClassID,
            need_initialization: bool,
        ) {
            assert!(need_initialization);
            match class_id {
                0 => unsafe {
                    let ptr = std::mem::transmute::<_, *mut u64>(isolate);
                    (*ptr.offset(3)) = 0xdeadbeaf_beafdead;
                },
                1 => unsafe {
                    let ptr = std::mem::transmute::<_, *mut u64>(isolate);
                    (*ptr.offset(4)) = 0xdeadbeaf_beafdead;
                },
                _ => {}
            }
        }

        let args: [u64; 1] = [0];
        extern "C" fn allocate_args(isolate: &mut Isolate, args: &mut Vec<String>) -> JavaArrayRef {
            unsafe {
                let isolate_ptr = std::mem::transmute::<_, *mut u64>(isolate);
                (*isolate_ptr.offset(5)) = 0xdeadbeaf_beafdead;
                let args_ptr = std::mem::transmute::<_, *mut u64>(args);
                (*args_ptr) = 0xbeaf;
            }
            null_mut()
        }

        main.static_methods.push(StaticMethodInfo {
            symbol: codegen.main_class_symbol.clone(),
            ptr: Some(main_fn as *const u8),
        });

        codegen.add_class(dummy_java_lang_object());
        codegen.add_class(main);
        codegen.done_compilation();

        codegen.cc.execution_engine.add_global_mapping(
            &codegen
                .cc
                .module
                .get_function("__yajvm_new_class_object")
                .unwrap(),
            new_class_object as usize,
        );

        codegen.cc.execution_engine.add_global_mapping(
            &codegen
                .cc
                .module
                .get_function("__yajvm_get_class_object")
                .unwrap(),
            get_class_object as usize,
        );

        codegen.cc.execution_engine.add_global_mapping(
            &codegen
                .cc
                .module
                .get_function("__yajvm_new_instance")
                .unwrap(),
            Isolate::new_instance as usize,
        );

        codegen.cc.execution_engine.add_global_mapping(
            &codegen.cc.module.get_function("allocate_args").unwrap(),
            allocate_args as usize,
        );

        codegen.cc.module.print_to_stderr();
        codegen.cc.module.verify().unwrap();

        let f: JitFunction<'_, unsafe extern "C" fn(*const u8, *const u8)> =
            unsafe { codegen.cc.execution_engine.get_function("main").unwrap() };
        unsafe {
            f.call(
                std::mem::transmute::<_, *const u8>(&isolate[0]),
                std::mem::transmute::<_, *const u8>(&args[0]),
            )
        }

        assert_eq!(isolate[0], 0xdeadbeaf);
        assert_eq!(isolate[1], 0xdeadbeaf_beafdead);
        assert_eq!(isolate[2], 0xdeadbeaf_beafdead);
        assert_eq!(isolate[3], 0xdeadbeaf_beafdead);
        assert_eq!(isolate[4], 0xdeadbeaf_beafdead);
        assert_eq!(isolate[5], 0xdeadbeaf_beafdead);
        assert_eq!(args[0], 0xbeaf);
    }

    #[test]
    fn test_build_inheritance_tree() {
        let mut codegen = CodeGen::new("Main");
        codegen.add_class(CompiledClass::new("java/lang/Object", None));
        codegen.add_class(CompiledClass::new("a", None));
        codegen.add_class(CompiledClass::new("c", Some("b".to_string())));
        codegen.add_class(CompiledClass::new("b", Some("a".to_string())));

        codegen.assign_class_ids();
        codegen.build_inheritance_tree();
        assert_eq!(codegen.java_lang_object_class_id, 3);
        assert_eq!(codegen.class_id("a"), 0);
        assert_eq!(codegen.class_id("b"), 1);
        assert_eq!(codegen.class_id("c"), 2);
        assert_eq!(codegen.class_parents[0], 3);
        assert_eq!(codegen.class_parents[1], 0);
        assert_eq!(codegen.class_parents[2], 1);
        assert_eq!(codegen.class_parents[3], 3);
    }

    #[test]
    fn test_add_class() {
        let mut codegen = CodeGen::new("Main");
        let mut class = CompiledClass::new("MyClass", None);
        let isolate: [u64; 2] = [0, 1];

        extern "C" fn clinit(isolate: &mut Isolate) {
            unsafe {
                let ptr = std::mem::transmute::<_, *mut u64>(isolate);
                (*ptr) = 0xdeadbeaf;
            }
        }
        class.clinit = Some(clinit);
        extern "C" fn static_method(isolate: &mut Isolate) {
            unsafe {
                let ptr = std::mem::transmute::<_, *mut u64>(isolate);
                (*(ptr.offset(1))) = 0xdeadbeaf_beafdead;
            }
        }
        class.static_methods.push(StaticMethodInfo {
            symbol: "MyClass.staticMethod:()V".to_string(),
            ptr: Some(static_method as *const u8),
        });

        codegen.add_class(class);

        let main = codegen.declare_main_function();
        let blk = codegen.cc.context.append_basic_block(main, "entry");
        codegen.cc.builder.position_at_end(blk);

        let isolate_ptr = main.get_nth_param(0).unwrap().into_pointer_value();
        let clinit = codegen.cc.module.get_function("MyClass.clinit").unwrap();
        codegen
            .cc
            .builder
            .build_call(clinit, &[isolate_ptr.into()], "call clinit");
        let static_method_fn = codegen
            .cc
            .module
            .get_function("MyClass.staticMethod:()V")
            .unwrap();
        codegen.cc.builder.build_call(
            static_method_fn,
            &[isolate_ptr.into()],
            "call static method",
        );
        codegen.cc.builder.build_return(None);

        codegen.cc.module.print_to_stderr();
        codegen.cc.module.verify().unwrap();

        let func = {
            let fn_ptr = codegen
                .cc
                .execution_engine
                .get_function_address("main")
                .unwrap();
            unsafe { std::mem::transmute::<_, unsafe extern "C" fn(*mut u8)>(fn_ptr) }
        };
        unsafe { func(std::mem::transmute::<_, *mut u8>(&isolate[0])) }
        assert_eq!(isolate[0], 0xdeadbeaf);
        assert_eq!(isolate[1], 0xdeadbeaf_beafdead);
    }

    #[test]
    fn test_compile_class_object_allocation() {
        let mut codegen = CodeGen::new("Main");
        codegen.add_class(dummy_java_lang_object());

        codegen.add_class(CompiledClass::new("MyClass", None));
        codegen.assign_class_ids();
        codegen.build_inheritance_tree();
        codegen.construct_vtables();

        let main = codegen.declare_main_function();
        let blk = codegen.cc.context.append_basic_block(main, "entry");
        codegen.cc.builder.position_at_end(blk);

        let clinit = {
            let const_ptr = codegen.cc.ptr_sized_type.const_int(0xdeadbeaf, false);
            codegen
                .cc
                .builder
                .build_int_to_ptr(const_ptr, codegen.cc.void_ptr, "clinit")
        };
        codegen.compile_class_object_allocation(
            main.get_nth_param(0).unwrap().into_pointer_value(),
            "MyClass",
            50,
            100,
            clinit,
        );
        codegen.cc.builder.build_return(None);

        let isolate: [*const u8; 1] = [null_mut()];
        unsafe extern "C" fn new_class_object(
            isolate: *mut *const u8,
            class_id: ClassID,
            static_fields_size: u32,
            instance_size: u32,
            vtable: *const u8,
            clinit: *const u8,
        ) {
            assert_eq!(class_id as usize, 0);
            assert_eq!(static_fields_size, 50);
            assert_eq!(instance_size, 100);
            (*isolate) = vtable;
            assert_eq!(clinit as usize, 0xdeadbeaf);
        }

        codegen
            .cc
            .execution_engine
            .add_global_mapping(&codegen.cc.new_class_object_fn, new_class_object as usize);
        add_get_global_function(&codegen.cc, "vtable###MyClass");

        unsafe {
            let raw = codegen
                .cc
                .execution_engine
                .get_function_address("main")
                .unwrap();
            let func =
                std::mem::transmute::<_, unsafe extern "C" fn(*const u8, *const u8) -> usize>(raw);
            func(isolate.as_ptr() as *const u8, null_mut());
        };

        codegen.cc.module.print_to_stderr();
        codegen.cc.module.verify().unwrap();

        let vtable_ptr = call_get_global_function(&codegen.cc, "get_vtable###MyClass");
        assert_eq!(vtable_ptr, unsafe {
            std::mem::transmute::<_, usize>(isolate[0])
        });
    }

    #[test]
    fn test_resolve_static_field_offsets() {
        let mut codegen = CodeGen::new("Main");
        let mut a = CompiledClass::new("a", None);
        a.static_fields = vec!["foo".to_string(), "bar".to_string(), "foobar".to_string()];
        codegen.add_class(a);
        let mut b = CompiledClass::new("b", None);
        b.static_fields = vec!["cat".to_string(), "dog".to_string()];
        codegen.add_class(b);

        fn add_get_static_field_offset_fn(cc: &mut CodegenContext, class_name: &str, field: &str) {
            let fn_type = cc.i32_type.fn_type(&[], false);
            let fn_value = cc.module.add_function(
                format!("get_{}_{}", class_name, field).as_str(),
                fn_type,
                None,
            );
            let entry = cc.context.append_basic_block(fn_value, "entry");
            cc.builder.position_at_end(entry);
            let class_id =
                cc.get_static_filed_offset_value(&class_name.to_string(), &field.to_string());
            cc.builder
                .build_return(Some(&class_id.as_basic_value_enum()));
        }

        add_get_static_field_offset_fn(&mut codegen.cc, "a", "foo");
        add_get_static_field_offset_fn(&mut codegen.cc, "a", "bar");
        add_get_static_field_offset_fn(&mut codegen.cc, "a", "foobar");
        add_get_static_field_offset_fn(&mut codegen.cc, "b", "dog");
        add_get_static_field_offset_fn(&mut codegen.cc, "b", "cat");

        codegen.resolve_static_field_offsets();
        codegen.cc.module.print_to_stderr();
        codegen.cc.module.verify().unwrap();

        fn call_get_static_field_offset(
            cc: &CodegenContext,
            class_name: &str,
            field_name: &str,
        ) -> u32 {
            let fn_ptr = cc
                .execution_engine
                .get_function_address(format!("get_{}_{}", class_name, field_name).as_str())
                .unwrap();
            let fn_ptr = unsafe { std::mem::transmute::<_, unsafe extern "C" fn() -> u32>(fn_ptr) };
            unsafe { fn_ptr() }
        }

        assert_eq!(0, call_get_static_field_offset(&codegen.cc, "a", "bar"));
        assert_eq!(8, call_get_static_field_offset(&codegen.cc, "a", "foo"));
        assert_eq!(16, call_get_static_field_offset(&codegen.cc, "a", "foobar"));
        assert_eq!(0, call_get_static_field_offset(&codegen.cc, "b", "cat"));
        assert_eq!(8, call_get_static_field_offset(&codegen.cc, "b", "dog"));
    }

    #[test]
    fn test_assign_class_ids() {
        let mut codegen = CodeGen::new("Main");
        codegen.add_class(CompiledClass::new("a", None));
        codegen.add_class(CompiledClass::new("b", None));
        codegen.add_class(CompiledClass::new("c", None));
        fn add_get_class_id_fn(cc: &mut CodegenContext, class_name: &str) {
            let fn_type = cc.i32_type.fn_type(&[], false);
            let fn_value =
                cc.module
                    .add_function(format!("get_{}", class_name).as_str(), fn_type, None);
            let entry = cc.context.append_basic_block(fn_value, "entry");
            cc.builder.position_at_end(entry);
            let class_id = cc.get_class_id_value(&class_name.to_string());
            cc.builder
                .build_return(Some(&class_id.as_basic_value_enum()));
        }

        add_get_class_id_fn(&mut codegen.cc, "a");
        add_get_class_id_fn(&mut codegen.cc, "b");
        add_get_class_id_fn(&mut codegen.cc, "c");

        codegen.assign_class_ids();
        assert_eq!(0, codegen.class_id("a"));
        assert_eq!(1, codegen.class_id("b"));
        assert_eq!(2, codegen.class_id("c"));

        codegen.cc.module.print_to_stderr();
        codegen.cc.module.verify().unwrap();

        fn call_get_class_id_fn(cc: &CodegenContext, class_name: &str) -> u32 {
            let fn_ptr = cc
                .execution_engine
                .get_function_address(format!("get_{}", class_name).as_str())
                .unwrap();
            let fn_ptr = unsafe { std::mem::transmute::<_, unsafe extern "C" fn() -> u32>(fn_ptr) };
            unsafe { fn_ptr() }
        }

        let a = call_get_class_id_fn(&codegen.cc, "a");
        let b = call_get_class_id_fn(&codegen.cc, "b");
        let c = call_get_class_id_fn(&codegen.cc, "c");
        assert_eq!(0, a);
        assert_eq!(1, b);
        assert_eq!(2, c);
    }

    #[test]
    fn test_construct_vtables() {
        let mut codegen = CodeGen::new("Main");
        codegen.add_class(dummy_java_lang_object());

        // Forward reference to the vtable should be handled correctly.
        codegen.cc.module.add_global(
            codegen.cc.void_ptr.array_type(0),
            None,
            "forward_declared_vtable###foo/bar/MyClass",
        );
        add_get_global_function(&codegen.cc, "forward_declared_vtable###foo/bar/MyClass");

        let mut rust_class = CompiledClass::new("foo/bar/MyClass", None);
        fn foo() {}
        fn bar() {}
        fn foobar() {}
        fn to_string() {}
        rust_class.virtual_methods = vec![
            VirtualMethodInfo {
                symbol: "java/lang/Object.toString:()Ljava/lang/String;@foo/bar/MyClass"
                    .to_string(),
                ptr: Some(to_string as *const u8),
                overrides: Some("java/lang/Object.toString:()Ljava/lang/String;".to_string()),
            },
            VirtualMethodInfo {
                symbol: "foo/bar/MyClass.foobar:()V".to_string(),
                ptr: Some(foobar as *const u8),
                overrides: None,
            },
            VirtualMethodInfo {
                symbol: "foo/bar/MyClass.bar:()V".to_string(),
                ptr: Some(bar as *const u8),
                overrides: None,
            },
            VirtualMethodInfo {
                symbol: "foo/bar/MyClass.foo:()V".to_string(),
                ptr: Some(foo as *const u8),
                overrides: None,
            },
        ];
        codegen.add_class(rust_class);
        let mut naive_class = CompiledClass::new("foo/bar/Native", None);
        naive_class.virtual_methods = vec![
            VirtualMethodInfo {
                symbol: "foo/bar/Native.ZZZZZZZZZZZZZZZ".to_string(),
                ptr: None,
                overrides: None,
            },
            VirtualMethodInfo {
                symbol: "foo/bar/Native.FFFFFFFFFF".to_string(),
                ptr: None,
                overrides: None,
            },
            VirtualMethodInfo {
                symbol: "foo/bar/Native.AAA".to_string(),
                ptr: None,
                overrides: None,
            },
            VirtualMethodInfo {
                symbol: "foo/bar/Native.CCC".to_string(),
                ptr: None,
                overrides: None,
            },
            VirtualMethodInfo {
                symbol: "foo/bar/Native.BBB".to_string(),
                ptr: None,
                overrides: None,
            },
        ];
        for m in &naive_class.virtual_methods {
            let f = codegen.cc.module.add_function(
                m.symbol.as_str(),
                codegen.cc.context.void_type().fn_type(&[], false),
                None,
            );
            let blk = codegen.cc.context.append_basic_block(f, "entry");
            codegen.cc.builder.position_at_end(blk);
            codegen.cc.builder.build_return(None);
        }
        codegen.add_class(naive_class);

        fn add_get_virtual_method_offset_fn(cc: &mut CodegenContext, symbol: &str) {
            let fn_type = cc.i32_type.fn_type(&[], false);
            let fn_value =
                cc.module
                    .add_function(format!("get_{}", symbol).as_str(), fn_type, None);
            let entry = cc.context.append_basic_block(fn_value, "entry");
            cc.builder.position_at_end(entry);
            let class_id = cc.get_virtual_method_offset_value(&symbol.to_string());
            cc.builder
                .build_return(Some(&class_id.as_basic_value_enum()));
        }

        add_get_virtual_method_offset_fn(
            &mut codegen.cc,
            "java/lang/Object.toString:()Ljava/lang/String;",
        );
        add_get_virtual_method_offset_fn(&mut codegen.cc, "foo/bar/MyClass.foobar:()V");
        add_get_virtual_method_offset_fn(&mut codegen.cc, "foo/bar/MyClass.foo:()V");
        add_get_virtual_method_offset_fn(&mut codegen.cc, "foo/bar/MyClass.bar:()V");
        add_get_virtual_method_offset_fn(&mut codegen.cc, "foo/bar/Native.AAA");
        add_get_virtual_method_offset_fn(&mut codegen.cc, "foo/bar/Native.BBB");
        add_get_virtual_method_offset_fn(&mut codegen.cc, "foo/bar/Native.CCC");

        codegen.assign_class_ids();
        codegen.build_inheritance_tree();
        codegen.construct_vtables();

        add_get_global_function(&codegen.cc, "vtable###foo/bar/Native");

        codegen.cc.module.print_to_stderr();
        codegen.cc.module.verify().unwrap();

        assert_eq!(3, codegen.vtables.len());
        assert_eq!(
            codegen
                .vtable_offsets
                .get("java/lang/Object.toString:()Ljava/lang/String;")
                .unwrap(),
            &0
        );
        assert_eq!(
            codegen
                .vtable_offsets
                .get("foo/bar/MyClass.foo:()V")
                .unwrap(),
            &2
        );
        assert_eq!(
            codegen
                .vtable_offsets
                .get("foo/bar/MyClass.bar:()V")
                .unwrap(),
            &1
        );
        assert_eq!(
            codegen
                .vtable_offsets
                .get("foo/bar/MyClass.foobar:()V")
                .unwrap(),
            &3
        );
        assert_eq!(
            codegen.vtable_offsets.get("foo/bar/Native.AAA").unwrap(),
            &1
        );
        assert_eq!(
            codegen.vtable_offsets.get("foo/bar/Native.BBB").unwrap(),
            &2
        );
        assert_eq!(
            codegen.vtable_offsets.get("foo/bar/Native.CCC").unwrap(),
            &3
        );

        let vtable_ptr =
            call_get_global_function(&codegen.cc, "get_forward_declared_vtable###foo/bar/MyClass");
        let vtable = unsafe { std::mem::transmute::<_, &[usize; 4]>(vtable_ptr) };
        assert_ne!(0, vtable_ptr);
        assert_eq!(vtable[0], to_string as usize);
        assert_eq!(vtable[1], bar as usize);
        assert_eq!(vtable[2], foo as usize);
        assert_eq!(vtable[3], foobar as usize);

        let vtable_ptr = call_get_global_function(&codegen.cc, "get_vtable###foo/bar/Native");
        assert_ne!(0, vtable_ptr);
        assert_eq!(read_vtable(vtable_ptr, 0), to_string_default as usize);
        assert_eq!(
            read_vtable(vtable_ptr, 1),
            get_address_of_function(&codegen.cc, "foo/bar/Native.AAA")
        );
        assert_eq!(
            read_vtable(vtable_ptr, 2),
            get_address_of_function(&codegen.cc, "foo/bar/Native.BBB")
        );
        assert_eq!(
            read_vtable(vtable_ptr, 3),
            get_address_of_function(&codegen.cc, "foo/bar/Native.CCC")
        );
        assert_eq!(
            read_vtable(vtable_ptr, 4),
            get_address_of_function(&codegen.cc, "foo/bar/Native.FFFFFFFFFF")
        );
        assert_eq!(
            read_vtable(vtable_ptr, 5),
            get_address_of_function(&codegen.cc, "foo/bar/Native.ZZZZZZZZZZZZZZZ")
        );

        fn call_get_virtual_method_offset(cc: &CodegenContext, symbol: &str) -> u32 {
            let fn_ptr = cc
                .execution_engine
                .get_function_address(format!("get_{}", symbol).as_str())
                .unwrap();
            let fn_ptr = unsafe { std::mem::transmute::<_, unsafe extern "C" fn() -> u32>(fn_ptr) };
            unsafe { fn_ptr() }
        }

        assert_eq!(
            0,
            call_get_virtual_method_offset(
                &codegen.cc,
                "java/lang/Object.toString:()Ljava/lang/String;",
            )
        );
        assert_eq!(
            3,
            call_get_virtual_method_offset(&codegen.cc, "foo/bar/MyClass.foobar:()V",)
        );
        assert_eq!(
            1,
            call_get_virtual_method_offset(&codegen.cc, "foo/bar/MyClass.bar:()V")
        );
        assert_eq!(
            2,
            call_get_virtual_method_offset(&codegen.cc, "foo/bar/MyClass.foo:()V")
        );
        assert_eq!(
            1,
            call_get_virtual_method_offset(&codegen.cc, "foo/bar/Native.AAA")
        );
        assert_eq!(
            2,
            call_get_virtual_method_offset(&codegen.cc, "foo/bar/Native.BBB")
        );
        assert_eq!(
            3,
            call_get_virtual_method_offset(&codegen.cc, "foo/bar/Native.CCC")
        );
    }

    fn get_address_of_function(ctx: &CodegenContext, name: &str) -> usize {
        let fn_ptr = ctx.execution_engine.get_function_address(name).unwrap();
        fn_ptr
    }

    fn read_vtable(ptr: usize, index: usize) -> usize {
        unsafe { *((ptr + index * size_of::<usize>()) as *const usize) }
    }

    fn add_get_global_function(cc: &CodegenContext, symbol: &str) {
        let fn_type = cc.ptr_sized_type.fn_type(&[cc.void_ptr.into()], false);
        let fn_value = cc
            .module
            .add_function(format!("get_{}", symbol).as_str(), fn_type, None);
        let entry = cc.context.append_basic_block(fn_value, "entry");
        cc.builder.position_at_end(entry);
        let g = cc.module.get_global(symbol).unwrap();
        let g_ptr = cc
            .builder
            .build_ptr_to_int(g.as_pointer_value(), cc.ptr_sized_type, "g_ptr");
        cc.builder.build_return(Some(&g_ptr));
    }

    fn call_get_global_function(cc: &CodegenContext, symbol: &str) -> usize {
        let fn_ptr = cc.execution_engine.get_function_address(symbol).unwrap();
        let fn_ptr = unsafe { std::mem::transmute::<_, unsafe extern "C" fn() -> usize>(fn_ptr) };
        unsafe { fn_ptr() }
    }

    fn to_string_default() {}

    fn dummy_java_lang_object() -> CompiledClass {
        let mut java_lang_object = CompiledClass::new("java/lang/Object", None);
        java_lang_object.virtual_methods.push(VirtualMethodInfo {
            symbol: "java/lang/Object.toString:()Ljava/lang/String;".to_string(),
            ptr: Some(to_string_default as *const u8),
            overrides: None,
        });
        java_lang_object
    }
}
