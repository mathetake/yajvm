use crate::codegen::descriptor;
use crate::codegen::descriptor::{
    parse_field_type_descriptor, parse_method_descriptor, BaseType, FieldType, MethodType,
};
#[allow(unused_imports)]
use bitflags::Flags;
#[warn(unused_imports)]
use std::collections::{HashMap, HashSet};

use crate::codegen::codegen_class_static_fields::load_class_obj_static_field_ptr;
use crate::codegen::codegen_context::CodegenContext;
use crate::compiled_class::{CompiledClass, StaticMethodInfo};
use crate::tracing::{insert_call_tracing_after, insert_call_tracing_before};
use classfile_parser::attribute_info::code_attribute_parser;
use classfile_parser::class_parser;
use classfile_parser::code_attribute::code_parser;
use classfile_parser::code_attribute::Instruction;
use classfile_parser::constant_info::ConstantInfo;
use classfile_parser::field_info::FieldAccessFlags;
use classfile_parser::method_info::{MethodAccessFlags, MethodInfo};
use classfile_parser::ClassFile;
use inkwell::basic_block::BasicBlock;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::FunctionValue;
use inkwell::values::{BasicValue, BasicValueEnum, PhiValue, PointerValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};

// https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html
// https://en.wikipedia.org/wiki/List_of_Java_bytecode_instructions

pub struct CompilationState<'ctx> {
    value_stack: Vec<BasicValueEnum<'ctx>>,
    locals: Vec<Option<BasicValueEnum<'ctx>>>,
    field_type_stack: Vec<FieldType>,
    param_count: usize,
    locals_field_types: Vec<Option<FieldType>>,
    function: Option<FunctionValue<'ctx>>,
    function_symbol: Option<String>,
    function_method_type: Option<MethodType>,
    is_static: bool,
    labels: HashMap<usize, BasicBlock<'ctx>>,
    label_field_type_stack: HashMap<usize, Vec<FieldType>>,
    label_phis: HashMap<BasicBlock<'ctx>, LabelPhis<'ctx>>,
    ignored_instructions: HashSet<usize>,
}

struct LabelPhis<'ctx> {
    local_phis: Vec<PhiValue<'ctx>>,
    stack_phis: Vec<PhiValue<'ctx>>,
}

impl<'ctx> CompilationState<'ctx> {
    fn push_value(&mut self, value: BasicValueEnum<'ctx>) {
        self.value_stack.push(value);
    }

    fn pop_value(&mut self) -> BasicValueEnum<'ctx> {
        self.value_stack.pop().unwrap()
    }

    fn reset(&mut self) {
        self.value_stack.truncate(0);
        self.locals.truncate(0);
        self.function = None;
        self.function_symbol = None;
        self.function_method_type = None;
        self.is_static = false;
        self.labels.clear();
        self.field_type_stack.truncate(0);
        self.locals_field_types.truncate(0);
        self.ignored_instructions.clear();
        self.label_field_type_stack.clear();
    }

    pub fn isolate_ptr(&self) -> PointerValue<'ctx> {
        self.function()
            .get_nth_param(0)
            .unwrap()
            .into_pointer_value()
    }

    pub fn local_field_types(&self) -> &Vec<Option<FieldType>> {
        &self.locals_field_types
    }

    pub fn function(&self) -> FunctionValue<'ctx> {
        self.function.unwrap()
    }

    pub fn value_stack_peek(&self) -> BasicValueEnum<'ctx> {
        self.value_stack.last().unwrap().clone()
    }

    pub fn function_method_type(&self) -> &MethodType {
        self.function_method_type.as_ref().unwrap()
    }

    pub fn function_symbol(&self) -> &String {
        self.function_symbol.as_ref().unwrap()
    }

    pub fn param_count(&self) -> usize {
        self.param_count
    }

    fn reserve_locals(&mut self, count: usize) {
        self.locals.resize(count, None);
        self.locals_field_types.resize(count, None);
    }

    fn initialize_local(
        &mut self,
        index: usize,
        field_type: FieldType,
        value: BasicValueEnum<'ctx>,
    ) {
        if index >= self.locals.len() {
            self.locals.resize(index + 1, None);
            self.locals_field_types.resize(index + 1, None);
        }

        if self.locals[index].is_none() {
            self.locals[index] = Some(value);
        }
        if self.locals_field_types[index].is_none() {
            self.locals_field_types[index] = Some(field_type);
        }
    }

    fn set_local(&mut self, index: usize, value: BasicValueEnum<'ctx>) {
        self.locals[index] = Some(value);
    }

    fn get_local(&self, index: usize) -> BasicValueEnum<'ctx> {
        self.locals[index].unwrap()
    }

    fn switch_to_block(&mut self, ctx: &CodegenContext<'ctx>, target_blk: BasicBlock<'ctx>) {
        let phis = self.label_phis.get(&target_blk).unwrap();
        for (i, phi) in phis.local_phis.iter().enumerate() {
            self.locals[i] = Some(phi.as_basic_value());
        }
        self.value_stack.clear();
        for (_, phi) in phis.stack_phis.iter().enumerate() {
            self.value_stack.push(phi.as_basic_value());
        }
        ctx.builder.position_at_end(target_blk);
    }

    fn set_locals_edges(&mut self, ctx: &CodegenContext<'ctx>, target_blk: BasicBlock<'ctx>) {
        let current_blk = ctx.builder.get_insert_block().unwrap();
        let phis = self.label_phis.get(&target_blk).unwrap();
        for (i, phi) in phis.local_phis.iter().enumerate() {
            let value = self.locals[i].unwrap();
            phi.add_incoming(&[(&value, current_blk)]);
        }
        for (i, phi) in phis.stack_phis.iter().enumerate() {
            let value = self.value_stack[i];
            phi.add_incoming(&[(&value, current_blk)]);
        }
    }
}

pub struct ClassFileCompiler {
    tracing_enabled: bool,
    class_name: String,
    super_class_name: String,
    class_file: ClassFile,
    class_static_fields: Vec<String>,
    static_methods: Vec<String>,
    virtual_methods: Vec<String>,
}

impl<'ctx> ClassFileCompiler {
    pub fn new(path: String, tracing_enabled: bool) -> Self {
        let classfile_bytes = std::fs::read(path.clone()).unwrap();
        let (_, class_file) = class_parser(&classfile_bytes).unwrap();

        let mut ret = Self {
            tracing_enabled,
            class_name: String::default(),
            super_class_name: String::default(),
            class_file,
            class_static_fields: Vec::default(),
            static_methods: Vec::default(),
            virtual_methods: Vec::default(),
        };

        let class_name = match ret.get_const(ret.class_file.this_class as usize) {
            ConstantInfo::Class(class_name) => ret.get_utf8_const(class_name.name_index as usize),
            _ => unreachable!(),
        };

        let super_class = match ret.get_const(ret.class_file.super_class as usize) {
            ConstantInfo::Class(class_name) => {
                Some(ret.get_utf8_const(class_name.name_index as usize))
            }
            _ => None,
        }
        .unwrap();

        ret.class_name = class_name;
        ret.super_class_name = super_class;
        ret
    }

    pub fn class_name(&self) -> String {
        self.class_name.clone()
    }

    pub fn as_class(&self) -> CompiledClass {
        let mut c = CompiledClass::new(
            self.class_name().as_str(),
            Some(self.super_class_name.clone()),
        );
        c.static_fields = self.class_static_fields.clone();

        for m in &self.static_methods {
            c.static_methods.push(StaticMethodInfo {
                symbol: m.clone(),
                ptr: None,
            });
        }
        c
    }

    pub fn methods(&self) -> &Vec<MethodInfo> {
        &self.class_file.methods
    }

    /// Get a constant from the constant pool.
    /// The index is 1-based as in the original JVM spec.
    fn get_const(&self, index: usize) -> &ConstantInfo {
        self.class_file.const_pool.get(index - 1).unwrap()
    }

    fn resolve_class_field(
        &self,
        class_index: usize,
        name_and_type_index: usize,
    ) -> (String, String, String) {
        let class_name = match self.get_const(class_index) {
            ConstantInfo::Class(class_name) => self.get_utf8_const(class_name.name_index as usize),
            _ => unreachable!(),
        };

        let (field_name, descriptor) = match self.get_const(name_and_type_index) {
            ConstantInfo::NameAndType(name_and_type) => {
                let name = self.get_utf8_const(name_and_type.name_index as usize);
                let descriptor_name = self.get_utf8_const(name_and_type.descriptor_index as usize);
                (name, descriptor_name)
            }
            _ => unreachable!(),
        };
        (class_name, field_name, descriptor)
    }

    fn get_utf8_const(&self, index: usize) -> String {
        match self.get_const(index) {
            ConstantInfo::Utf8(name) => name.utf8_string.clone(),
            _ => unreachable!(),
        }
    }

    pub fn initialize_class_object_info(&mut self) {
        for f in &self.class_file.fields {
            if f.access_flags.contains(FieldAccessFlags::STATIC) {
                let field_name = self.get_utf8_const(f.name_index as usize);
                self.class_static_fields.push(field_name);
            }
        }
    }

    pub fn compile_methods(&mut self, ctx: &mut CodegenContext<'ctx>) {
        let mut state = CompilationState {
            value_stack: Vec::new(),
            param_count: 0,
            locals: Vec::new(),
            function: None,
            function_symbol: None,
            is_static: false,
            field_type_stack: Vec::new(),
            labels: HashMap::new(),
            locals_field_types: Vec::new(),
            label_phis: HashMap::new(),
            ignored_instructions: HashSet::new(),
            function_method_type: None,
            label_field_type_stack: HashMap::new(),
        };

        for method in &self.class_file.methods {
            self.compile_method(ctx, &method, &mut state);
            if state.is_static {
                self.static_methods
                    .push(state.function_symbol.clone().unwrap());
            } else {
                self.virtual_methods
                    .push(state.function_symbol.clone().unwrap());
            }
            state.reset(); // Reuse the same state for all methods.
        }
    }

    fn compile_method(
        &self,
        ctx: &mut CodegenContext<'ctx>,
        method: &MethodInfo,
        state: &mut CompilationState<'ctx>,
    ) {
        self.analyze(ctx, method, state);
        self.build_phis(ctx, state);
        self.compile(ctx, method, state);
    }

    fn build_phis(&self, ctx: &CodegenContext<'ctx>, state: &mut CompilationState<'ctx>) {
        for (i, block) in &state.labels {
            let block = *block;
            let mut local_phis = Vec::new();
            let mut stack_phis = Vec::new();
            if *i == 0 {
                state.label_phis.insert(
                    block,
                    LabelPhis {
                        local_phis,
                        stack_phis,
                    },
                );
                continue; // Entry label shouldn't have phis.
            }
            ctx.builder.position_at_end(block);
            for (i, v) in state.locals.iter().enumerate() {
                if let Some(v) = v {
                    let phi = ctx.builder.build_phi(
                        v.get_type(),
                        format!("locals[{}]_at_{}", i, block.get_name().to_str().unwrap()).as_str(),
                    );
                    local_phis.push(phi);
                }
            }
            for (i, v) in state
                .label_field_type_stack
                .get(i)
                .unwrap()
                .iter()
                .enumerate()
            {
                let llvm_typ: BasicTypeEnum = ctx.llvm_type_from_field_type(&v);
                let phi = ctx.builder.build_phi(
                    llvm_typ,
                    format!("stack[{}]_at_{}", i, block.get_name().to_str().unwrap()).as_str(),
                );
                stack_phis.push(phi);
            }
            state.label_phis.insert(
                block,
                LabelPhis {
                    local_phis,
                    stack_phis,
                },
            );
        }
    }

    fn get_local_method_by_symbol(
        &self,
        ctx: &CodegenContext<'ctx>,
        method_name: &String,
        descriptor_str: &String,
        descriptor: &MethodType,
        is_static: bool,
    ) -> (FunctionValue<'ctx>, String) {
        let symbol = format!("{}.{}:{}", &self.class_name(), method_name, descriptor_str);
        let f = ctx.module.get_function(&symbol).unwrap_or_else(|| {
            let fn_type = ctx.llvm_function_type_from_method_type(&descriptor, is_static);
            let function = ctx.module.add_function(&symbol, fn_type, None);
            function
        });
        (f, symbol)
    }

    fn analyze_load_constant(&self, state: &mut CompilationState<'ctx>, index: u16) {
        match self.get_const(index as usize) {
            ConstantInfo::String(_) => {
                state
                    .field_type_stack
                    .push(FieldType::ObjectType("java/lang/String".to_string()));
            }
            ConstantInfo::Integer(_) => {
                state
                    .field_type_stack
                    .push(FieldType::BaseType(BaseType::Int));
            }
            ConstantInfo::Float(_) => {
                state
                    .field_type_stack
                    .push(FieldType::BaseType(BaseType::Float));
            }
            ConstantInfo::Long(_) => {
                state
                    .field_type_stack
                    .push(FieldType::BaseType(BaseType::Long));
            }
            ConstantInfo::Double(_) => {
                state
                    .field_type_stack
                    .push(FieldType::BaseType(BaseType::Double));
            }
            v => unreachable!("{:?}", v),
        };
    }

    fn analyze(
        &self,
        ctx: &CodegenContext<'ctx>,
        method: &MethodInfo,
        state: &mut CompilationState<'ctx>,
    ) {
        let method_name = match self.get_const(method.name_index as usize) {
            ConstantInfo::Utf8(name) => name.utf8_string.clone(),
            _ => unreachable!(),
        };

        let (descriptor, descriptor_str) = match self.get_const(method.descriptor_index as usize) {
            ConstantInfo::Utf8(name) => (
                parse_method_descriptor(&name.utf8_string),
                name.utf8_string.clone(),
            ),
            _ => unreachable!(),
        };

        let is_static = method.access_flags.contains(MethodAccessFlags::STATIC);

        let (function, function_symbol) = self.get_local_method_by_symbol(
            ctx,
            &method_name,
            &descriptor_str,
            &descriptor,
            is_static,
        );

        state.param_count = 0;
        let mut local_index = 0;
        for (i, p) in function
            .get_params()
            .iter()
            .skip(1 /* RtCtx is not Java level arg. */)
            .enumerate()
        {
            let typ = if is_static {
                descriptor.parameter_types[i].clone() as FieldType
            } else {
                if i == 0 {
                    FieldType::ObjectType(self.class_name().clone())
                } else {
                    descriptor.parameter_types[i - 1].clone() as FieldType
                }
            };

            state.initialize_local(local_index, typ.clone(), *p);
            state.param_count += 1;
            match typ {
                FieldType::BaseType(BaseType::Double) | FieldType::BaseType(BaseType::Long) => {
                    state.initialize_local(local_index, typ.clone(), *p);
                    local_index += 2;
                }
                _ => local_index += 1,
            }
        }

        let entry_block = ctx.context.append_basic_block(function, "entry");
        state.labels.insert(0, entry_block);

        state.function = Some(function);
        state.function_symbol = Some(function_symbol);
        state.function_method_type = Some(descriptor);

        assert!(
            state.field_type_stack.is_empty(),
            "{:?}",
            state.field_type_stack
        );

        // Analyze the code to find all the labels, plus the local variable types.
        const INT: FieldType = FieldType::BaseType(BaseType::Int);
        const LONG: FieldType = FieldType::BaseType(BaseType::Long);
        const FLOAT: FieldType = FieldType::BaseType(BaseType::Float);
        const DOUBLE: FieldType = FieldType::BaseType(BaseType::Double);
        let int_basic_type_enum: BasicTypeEnum = ctx.llvm_type_from_field_type(&INT);
        let long_basic_type_enum: BasicTypeEnum = ctx.llvm_type_from_field_type(&LONG);
        let float_basic_type_enum: BasicTypeEnum = ctx.llvm_type_from_field_type(&FLOAT);
        let double_basic_type_enum: BasicTypeEnum = ctx.llvm_type_from_field_type(&DOUBLE);
        for attr_info in &method.attributes {
            let (_, code_attr) = code_attribute_parser(&attr_info.info).unwrap();
            state.reserve_locals(code_attr.max_locals as usize);

            let (_, code) = code_parser(&code_attr.code).unwrap();
            for (addr, instr) in code.iter() {
                if let Some(stack) = state.label_field_type_stack.get(addr) {
                    // Swap the stack.
                    state.field_type_stack = stack.clone();
                }

                // println!("{}: {:?}: {:?}", addr, instr, state.field_type_stack);
                match instr {
                    Instruction::Aaload => {
                        state.field_type_stack.pop().unwrap(); // index.
                        let array_type = state.field_type_stack.pop().unwrap();
                        match array_type {
                            FieldType::ArrayType(array_type) => {
                                state.field_type_stack.push(*array_type);
                            }
                            _ => unreachable!(),
                        }
                    }
                    Instruction::Iload0
                    | Instruction::Iload1
                    | Instruction::Iload2
                    | Instruction::Iload3 => {
                        let index = match instr {
                            Instruction::Iload0 => 0,
                            Instruction::Iload1 => 1,
                            Instruction::Iload2 => 2,
                            Instruction::Iload3 => 3,
                            _ => unreachable!(),
                        };
                        state.field_type_stack.push(INT);
                        state.initialize_local(index, INT, int_basic_type_enum.const_zero());
                    }

                    Instruction::Iload(index) => {
                        state.field_type_stack.push(INT);
                        state.initialize_local(
                            *index as usize,
                            INT,
                            int_basic_type_enum.const_zero(),
                        );
                    }

                    Instruction::Lload0
                    | Instruction::Lload1
                    | Instruction::Lload2
                    | Instruction::Lload3 => {
                        let index = match instr {
                            Instruction::Lload0 => 0,
                            Instruction::Lload1 => 1,
                            Instruction::Lload2 => 2,
                            Instruction::Lload3 => 3,
                            _ => unreachable!(),
                        };
                        state
                            .field_type_stack
                            .push(FieldType::BaseType(BaseType::Long));
                        state.initialize_local(
                            index,
                            FieldType::BaseType(BaseType::Long),
                            ctx.i64_type.const_zero().into(),
                        );
                        state.initialize_local(
                            index + 1,
                            FieldType::BaseType(BaseType::Long),
                            ctx.i64_type.const_zero().into(),
                        );
                    }

                    Instruction::Lload(index) => {
                        let index = *index as usize;
                        state
                            .field_type_stack
                            .push(FieldType::BaseType(BaseType::Long));
                        state.initialize_local(
                            index,
                            FieldType::BaseType(BaseType::Long),
                            ctx.i64_type.const_zero().into(),
                        );
                        state.initialize_local(
                            index + 1,
                            FieldType::BaseType(BaseType::Long),
                            ctx.i64_type.const_zero().into(),
                        );
                    }

                    Instruction::Dload0
                    | Instruction::Dload1
                    | Instruction::Dload2
                    | Instruction::Dload3 => {
                        let index = match instr {
                            Instruction::Dload0 => 0,
                            Instruction::Dload1 => 1,
                            Instruction::Dload2 => 2,
                            Instruction::Dload3 => 3,
                            _ => unreachable!(),
                        };
                        state
                            .field_type_stack
                            .push(FieldType::BaseType(BaseType::Double));
                        state.initialize_local(
                            index,
                            FieldType::BaseType(BaseType::Double),
                            ctx.f64_type.const_zero().into(),
                        );
                        state.initialize_local(
                            index + 1,
                            FieldType::BaseType(BaseType::Double),
                            ctx.f64_type.const_zero().into(),
                        );
                    }

                    Instruction::Dload(index) => {
                        let index = *index as usize;
                        state
                            .field_type_stack
                            .push(FieldType::BaseType(BaseType::Double));
                        state.initialize_local(
                            index,
                            FieldType::BaseType(BaseType::Double),
                            ctx.f64_type.const_zero().into(),
                        );
                        state.initialize_local(
                            index + 1,
                            FieldType::BaseType(BaseType::Double),
                            ctx.f64_type.const_zero().into(),
                        );
                    }

                    Instruction::Fload0
                    | Instruction::Fload1
                    | Instruction::Fload2
                    | Instruction::Fload3 => {
                        let index = match instr {
                            Instruction::Fload0 => 0,
                            Instruction::Fload1 => 1,
                            Instruction::Fload2 => 2,
                            Instruction::Fload3 => 3,
                            _ => unreachable!(),
                        };
                        state
                            .field_type_stack
                            .push(FieldType::BaseType(BaseType::Float));
                        state.initialize_local(
                            index,
                            FieldType::BaseType(BaseType::Float),
                            ctx.f32_type.const_zero().into(),
                        );
                    }

                    Instruction::Fload(index) => {
                        state
                            .field_type_stack
                            .push(FieldType::BaseType(BaseType::Float));
                        state.initialize_local(
                            *index as usize,
                            FieldType::BaseType(BaseType::Float),
                            ctx.f32_type.const_zero().into(),
                        );
                    }

                    Instruction::Arraylength => {
                        state.field_type_stack.pop();
                        state.field_type_stack.push(INT);
                    }

                    Instruction::Iconst0
                    | Instruction::Iconst1
                    | Instruction::Iconst2
                    | Instruction::Iconst3
                    | Instruction::Iconst4
                    | Instruction::Iconst5
                    | Instruction::Iconstm1 => {
                        state.field_type_stack.push(INT);
                    }

                    Instruction::Fconst0 | Instruction::Fconst1 | Instruction::Fconst2 => {
                        state.field_type_stack.push(FLOAT);
                    }

                    Instruction::Lconst0 | Instruction::Lconst1 => {
                        state.field_type_stack.push(LONG);
                    }

                    Instruction::Dconst0 | Instruction::Dconst1 => {
                        state.field_type_stack.push(DOUBLE);
                    }

                    Instruction::Istore0
                    | Instruction::Istore1
                    | Instruction::Istore2
                    | Instruction::Istore3 => {
                        let index = match instr {
                            Instruction::Istore0 => 0,
                            Instruction::Istore1 => 1,
                            Instruction::Istore2 => 2,
                            Instruction::Istore3 => 3,
                            _ => unreachable!(),
                        };

                        let field_type = state.field_type_stack.pop().unwrap();
                        state.initialize_local(index, field_type, int_basic_type_enum.const_zero());
                    }

                    Instruction::Istore(index) => {
                        let field_type = state.field_type_stack.pop().unwrap();
                        state.initialize_local(
                            *index as usize,
                            field_type,
                            int_basic_type_enum.const_zero(),
                        );
                    }

                    Instruction::Lstore0
                    | Instruction::Lstore1
                    | Instruction::Lstore2
                    | Instruction::Lstore3 => {
                        let index = match instr {
                            Instruction::Lstore0 => 0,
                            Instruction::Lstore1 => 1,
                            Instruction::Lstore2 => 2,
                            Instruction::Lstore3 => 3,
                            _ => unreachable!(),
                        };

                        let field_type = state.field_type_stack.pop().unwrap();
                        state.initialize_local(
                            index,
                            field_type,
                            long_basic_type_enum.const_zero(),
                        );
                    }

                    Instruction::Lstore(index) => {
                        let field_type = state.field_type_stack.pop().unwrap();
                        state.initialize_local(
                            *index as usize,
                            field_type,
                            long_basic_type_enum.const_zero(),
                        );
                    }

                    Instruction::Fstore0
                    | Instruction::Fstore1
                    | Instruction::Fstore2
                    | Instruction::Fstore3 => {
                        let index = match instr {
                            Instruction::Fstore0 => 0,
                            Instruction::Fstore1 => 1,
                            Instruction::Fstore2 => 2,
                            Instruction::Fstore3 => 3,
                            _ => unreachable!(),
                        };

                        let field_type = state.field_type_stack.pop().unwrap();
                        state.initialize_local(
                            index,
                            field_type,
                            float_basic_type_enum.const_zero(),
                        );
                    }

                    Instruction::Fstore(index) => {
                        let field_type = state.field_type_stack.pop().unwrap();
                        state.initialize_local(
                            *index as usize,
                            field_type,
                            float_basic_type_enum.const_zero(),
                        );
                    }

                    Instruction::Dstore0
                    | Instruction::Dstore1
                    | Instruction::Dstore2
                    | Instruction::Dstore3 => {
                        let index = match instr {
                            Instruction::Dstore0 => 0,
                            Instruction::Dstore1 => 1,
                            Instruction::Dstore2 => 2,
                            Instruction::Dstore3 => 3,
                            _ => unreachable!(),
                        };

                        let field_type = state.field_type_stack.pop().unwrap();
                        state.initialize_local(
                            index,
                            field_type,
                            double_basic_type_enum.const_zero(),
                        );
                    }

                    Instruction::Dstore(index) => {
                        let field_type = state.field_type_stack.pop().unwrap();
                        state.initialize_local(
                            *index as usize,
                            field_type,
                            double_basic_type_enum.const_zero(),
                        );
                    }

                    Instruction::Bipush(_) => {
                        state.field_type_stack.push(INT);
                    }
                    Instruction::Sipush(_) => {
                        state.field_type_stack.push(INT);
                    }

                    Instruction::Invokevirtual(index) => {
                        let sig = {
                            let method_ref = match self.get_const(*index as usize) {
                                ConstantInfo::MethodRef(method_ref) => method_ref,
                                v => unreachable!("{:?}", v),
                            };
                            let (class_name, method_name, raw) = self.resolve_class_field(
                                method_ref.class_index as usize,
                                method_ref.name_and_type_index as usize,
                            );

                            match &*class_name {
                                "java/lang/Integer" => match &*method_name {
                                    "intValue" => {
                                        // Ignore this instruction.
                                        state.ignored_instructions.insert(*addr);
                                    }
                                    _ => unreachable!(),
                                },
                                _ => {}
                            }

                            parse_method_descriptor(&raw)
                        };
                        for _ in 0..sig.parameter_types.len() + 1
                        // +1 because it takes this pointer.
                        {
                            state.field_type_stack.pop();
                        }
                        if let Some(return_type) = sig.return_type {
                            state.field_type_stack.push(return_type);
                        }
                    }
                    Instruction::Invokespecial(index) => {
                        let method_ref = match self.get_const(*index as usize) {
                            ConstantInfo::MethodRef(method_ref) => method_ref,
                            v => unreachable!("{:?}", v),
                        };

                        let (_, _, descriptor) = self.resolve_class_field(
                            method_ref.class_index as usize,
                            method_ref.name_and_type_index as usize,
                        );

                        let sig = parse_method_descriptor(&descriptor);
                        // Special takes an implicit this pointer.
                        state.field_type_stack.pop();
                        // Then the parameters.
                        for _ in 0..sig.parameter_types.len() {
                            state.field_type_stack.pop();
                        }
                        if let Some(return_type) = sig.return_type {
                            state.field_type_stack.push(return_type);
                        }
                    }

                    Instruction::Getstatic(index) => {
                        let desc = {
                            let field_ref = match self.get_const(*index as usize) {
                                ConstantInfo::FieldRef(method_ref) => method_ref,
                                v => unreachable!("{:?}", v),
                            };

                            let (_, _, descriptor) = self.resolve_class_field(
                                field_ref.class_index as usize,
                                field_ref.name_and_type_index as usize,
                            );
                            parse_field_type_descriptor(&descriptor)
                        };
                        state.field_type_stack.push(desc);
                    }

                    Instruction::Putstatic(index) => {
                        let _ = {
                            let field_ref = match self.get_const(*index as usize) {
                                ConstantInfo::FieldRef(method_ref) => method_ref,
                                v => unreachable!("{:?}", v),
                            };

                            let (_, _, descriptor) = self.resolve_class_field(
                                field_ref.class_index as usize,
                                field_ref.name_and_type_index as usize,
                            );
                            parse_field_type_descriptor(&descriptor)
                        };
                        state.field_type_stack.pop();
                    }

                    Instruction::Iinc { index, value: _ } => {
                        state.initialize_local(
                            *index as usize,
                            INT,
                            int_basic_type_enum.const_zero(),
                        );
                    }
                    Instruction::Ldc(index) => self.analyze_load_constant(state, *index as u16),
                    Instruction::Ldc2W(index) => self.analyze_load_constant(state, *index),

                    Instruction::Invokestatic(index) => {
                        let method_ref = match self.get_const(*index as usize) {
                            ConstantInfo::MethodRef(method_ref) => method_ref,
                            v => unreachable!("{:?}", v),
                        };

                        let (class_name, method_name, descriptor) = self.resolve_class_field(
                            method_ref.class_index as usize,
                            method_ref.name_and_type_index as usize,
                        );

                        match &*class_name {
                            "java/lang/Integer" => match &*method_name {
                                "valueOf" => {
                                    // Ignore this instruction.
                                    state.ignored_instructions.insert(*addr);
                                }
                                _ => unreachable!(),
                            },
                            _ => {}
                        }

                        let sig = parse_method_descriptor(&descriptor);
                        // Then the parameters.
                        for _ in 0..sig.parameter_types.len() {
                            state.field_type_stack.pop();
                        }
                        if let Some(return_type) = sig.return_type {
                            state.field_type_stack.push(return_type);
                        }
                    }
                    Instruction::Imul
                    | Instruction::Iadd
                    | Instruction::Isub
                    | Instruction::Idiv => {
                        state.field_type_stack.pop();
                        state.field_type_stack.pop();
                        state.field_type_stack.push(INT);
                    }

                    Instruction::Lmul
                    | Instruction::Ladd
                    | Instruction::Lsub
                    | Instruction::Ldiv => {
                        state.field_type_stack.pop();
                        state.field_type_stack.pop();
                        state.field_type_stack.push(LONG);
                    }

                    Instruction::Fmul
                    | Instruction::Fadd
                    | Instruction::Fsub
                    | Instruction::Fdiv => {
                        state.field_type_stack.pop();
                        state.field_type_stack.pop();
                        state.field_type_stack.push(FLOAT);
                    }

                    Instruction::Dmul
                    | Instruction::Dadd
                    | Instruction::Dsub
                    | Instruction::Ddiv => {
                        state.field_type_stack.pop();
                        state.field_type_stack.pop();
                        state.field_type_stack.push(DOUBLE);
                    }

                    Instruction::I2b | Instruction::I2c | Instruction::I2s => {
                        // This is just rounding the integer to a byte at runtime.
                        state.field_type_stack.pop();
                        state.field_type_stack.push(INT);
                    }

                    Instruction::Goto(diff) => {
                        self.ensure_label_with_offset(ctx, state, *addr, *diff);
                    }

                    Instruction::Ireturn | Instruction::Areturn => {
                        state.field_type_stack.pop().unwrap();
                    }

                    Instruction::IfIcmpge(diff)
                    | Instruction::IfIcmpne(diff)
                    | Instruction::IfIcmple(diff)
                    | Instruction::IfIcmplt(diff)
                    | Instruction::IfIcmpeq(diff)
                    | Instruction::IfIcmpgt(diff) => {
                        state.field_type_stack.pop();
                        state.field_type_stack.pop();
                        let addr = *addr;
                        self.ensure_label_with_offset(ctx, state, addr, *diff);
                        // Next instruction is the "else" target.
                        self.ensure_label_with_offset(ctx, state, addr, 3);
                    }

                    Instruction::Lcmp => {
                        state.field_type_stack.pop();
                        state.field_type_stack.pop();
                        state.field_type_stack.push(INT);
                    }

                    Instruction::Fcmpl
                    | Instruction::Fcmpg
                    | Instruction::Dcmpl
                    | Instruction::Dcmpg => {
                        state.field_type_stack.pop();
                        state.field_type_stack.pop();
                        state.field_type_stack.push(INT);
                    }

                    Instruction::Ifne(diff)
                    | Instruction::Ifeq(diff)
                    | Instruction::Ifgt(diff)
                    | Instruction::Ifge(diff)
                    | Instruction::Ifle(diff) => {
                        state.field_type_stack.pop();
                        let addr = *addr;
                        self.ensure_label_with_offset(ctx, state, addr, *diff);
                        // Next instruction is the "else" target.
                        self.ensure_label_with_offset(ctx, state, addr, 3);
                    }

                    Instruction::Return => {
                        if function.get_type().get_return_type().is_some() {
                            state.field_type_stack.pop();
                        }
                    }

                    Instruction::Newarray(atype) => {
                        // Array Type	atype
                        // T_BOOLEAN	4
                        // T_CHAR	5
                        // T_FLOAT	6
                        // T_DOUBLE	7
                        // T_BYTE	8
                        // T_SHORT	9
                        // T_INT	10
                        // T_LONG	11
                        let base_type = match atype {
                            4 => BaseType::Boolean,
                            5 => BaseType::Char,
                            6 => BaseType::Float,
                            7 => BaseType::Double,
                            8 => BaseType::Byte,
                            9 => BaseType::Short,
                            10 => BaseType::Int,
                            11 => BaseType::Long,
                            _ => unreachable!(),
                        };
                        state.field_type_stack.pop(); // The size of array.
                        state.field_type_stack.push(FieldType::ArrayType(Box::new(
                            FieldType::BaseType(base_type),
                        )));
                    }

                    Instruction::Astore0
                    | Instruction::Astore1
                    | Instruction::Astore2
                    | Instruction::Astore3 => {
                        let index = match instr {
                            Instruction::Astore0 => 0,
                            Instruction::Astore1 => 1,
                            Instruction::Astore2 => 2,
                            Instruction::Astore3 => 3,
                            _ => unreachable!(),
                        };
                        let field_type = state.field_type_stack.pop().unwrap();
                        let local_type: BasicTypeEnum = ctx.llvm_type_from_field_type(&field_type);
                        state.initialize_local(index, field_type, local_type.const_zero());
                    }

                    Instruction::Aload0
                    | Instruction::Aload1
                    | Instruction::Aload2
                    | Instruction::Aload3 => {
                        let index = match instr {
                            Instruction::Aload0 => 0,
                            Instruction::Aload1 => 1,
                            Instruction::Aload2 => 2,
                            Instruction::Aload3 => 3,
                            _ => unreachable!(),
                        };
                        let filed_type = state.locals_field_types[index].clone().unwrap();
                        state.field_type_stack.push(filed_type);
                    }

                    Instruction::Iastore
                    | Instruction::Bastore
                    | Instruction::Sastore
                    | Instruction::Castore
                    | Instruction::Lastore
                    | Instruction::Fastore
                    | Instruction::Dastore => {
                        state.field_type_stack.pop(); // Array ref.
                        state.field_type_stack.pop(); // Index.
                        state.field_type_stack.pop(); // Value.
                    }

                    Instruction::Iaload
                    | Instruction::Baload
                    | Instruction::Saload
                    | Instruction::Caload => {
                        state.field_type_stack.pop(); // Array ref.
                        state.field_type_stack.pop(); // Index.
                        state.field_type_stack.push(INT);
                    }

                    Instruction::Laload => {
                        state.field_type_stack.pop(); // Array ref.
                        state.field_type_stack.pop(); // Index.
                        state.field_type_stack.push(LONG);
                    }

                    Instruction::Faload => {
                        state.field_type_stack.pop(); // Array ref.
                        state.field_type_stack.pop(); // Index.
                        state.field_type_stack.push(FLOAT);
                    }

                    Instruction::Daload => {
                        state.field_type_stack.pop(); // Array ref.
                        state.field_type_stack.pop(); // Index.
                        state.field_type_stack.push(DOUBLE);
                    }

                    _ => {
                        panic!("{}: {:?}", addr, instr)
                    }
                }
            }
        }

        assert!(
            state.field_type_stack.is_empty(),
            "{:?}",
            state.field_type_stack
        );
    }

    fn ensure_label_with_offset(
        &self,
        ctx: &CodegenContext<'ctx>,
        state: &mut CompilationState<'ctx>,
        current_addr: usize,
        offset: i16,
    ) {
        let target_addr = (current_addr as isize + offset as isize) as usize;
        // If the target_addr doesn't have a label assigned, create it.
        if !state.labels.contains_key(&target_addr) {
            let basic_block = ctx
                .context
                .append_basic_block(state.function(), format!("l{}", target_addr).as_str());
            state.labels.insert(target_addr, basic_block);
            state
                .label_field_type_stack
                .insert(target_addr, state.field_type_stack.clone());
        }
    }

    fn load_constant(
        &self,
        ctx: &mut CodegenContext<'ctx>,
        state: &mut CompilationState<'ctx>,
        index: u16,
    ) {
        match self.get_const(index as usize) {
            ConstantInfo::String(str) => {
                let str = self.get_utf8_const(str.string_index as usize);
                let global = ctx.get_const_string_global(&str);
                state.push_value(global.as_basic_value_enum());
            }
            ConstantInfo::Integer(value) => {
                let int = ctx.i32_type.const_int(value.value as u64, false);
                state.push_value(int.into());
            }
            ConstantInfo::Long(value) => {
                let long = ctx.i64_type.const_int(value.value as u64, false);
                state.push_value(long.into());
            }
            ConstantInfo::Float(value) => {
                let float = ctx.f32_type.const_float(value.value as f64);
                state.push_value(float.into());
            }
            ConstantInfo::Double(value) => {
                let double = ctx.f64_type.const_float(value.value);
                state.push_value(double.into());
            }
            v => unreachable!("{:?}", v),
        };
    }

    fn label_with_offset(
        &self,
        state: &mut CompilationState<'ctx>,
        current_addr: usize,
        offset: i16,
    ) -> BasicBlock<'ctx> {
        let target_addr = (current_addr as isize + offset as isize) as usize;
        state.labels[&target_addr]
    }

    fn compile(
        &self,
        ctx: &mut CodegenContext<'ctx>,
        method: &MethodInfo,
        state: &mut CompilationState<'ctx>,
    ) {
        for attr_info in &method.attributes {
            let (_, code_attr) = code_attribute_parser(&attr_info.info).unwrap();
            let (_, code) = code_parser(&code_attr.code).unwrap();
            let mut terminated = true;
            for (addr, instr) in code {
                // println!("stack: {:?}\n\t{}: {:?}", state.value_stack, addr, instr);

                let label = state.labels.contains_key(&addr);
                if label {
                    let target_blk = state.labels[&addr];
                    if !terminated {
                        // Insert jump to the new label as "fallthrough".
                        state.set_locals_edges(ctx, target_blk);
                        ctx.builder.build_unconditional_branch(target_blk);
                    }
                    state.switch_to_block(ctx, target_blk);
                }

                if addr == 0 && self.tracing_enabled {
                    insert_call_tracing_before(ctx, state);
                }

                if state.ignored_instructions.contains(&addr) {
                    continue;
                }

                terminated = false;
                match instr {
                    // ---- Allocations ----
                    Instruction::Newarray(atype) => {
                        // Array Type	atype
                        // T_BOOLEAN	4
                        // T_CHAR	5
                        // T_FLOAT	6
                        // T_DOUBLE	7
                        // T_BYTE	8
                        // T_SHORT	9
                        // T_INT	10
                        // T_LONG	11
                        let f = match atype {
                            4 => ctx.new_boolean_array_fn,
                            5 => ctx.new_char_array_fn,
                            6 => ctx.new_float_array_fn,
                            7 => ctx.new_double_array_fn,
                            8 => ctx.new_byte_array_fn,
                            9 => ctx.new_short_array_fn,
                            10 => ctx.new_int_array_fn,
                            11 => ctx.new_long_array_fn,
                            _ => unreachable!(),
                        };

                        let size = state.pop_value();
                        let array_ptr = ctx
                            .builder
                            .build_call(f, &[state.isolate_ptr().into(), size.into()], "array_ptr")
                            .try_as_basic_value()
                            .left()
                            .unwrap();
                        state.push_value(array_ptr.into());
                    }

                    // ---- consts ----
                    Instruction::Iconstm1 => {
                        state.push_value(ctx.i32_type.const_int((-1i64) as u64, true).into());
                    }
                    Instruction::Iconst0 => {
                        state.push_value(ctx.i32_type.const_zero().into());
                    }
                    Instruction::Iconst1 => {
                        state.push_value(ctx.i32_type.const_int(1, false).into());
                    }
                    Instruction::Iconst2 => {
                        state.push_value(ctx.i32_type.const_int(2, false).into());
                    }
                    Instruction::Iconst3 => {
                        state.push_value(ctx.i32_type.const_int(3, false).into());
                    }
                    Instruction::Iconst4 => {
                        state.push_value(ctx.i32_type.const_int(4, false).into());
                    }
                    Instruction::Iconst5 => {
                        state.push_value(ctx.i32_type.const_int(5, false).into());
                    }
                    Instruction::Fconst0 => {
                        state.push_value(ctx.f32_type.const_zero().into());
                    }
                    Instruction::Fconst1 => {
                        state.push_value(ctx.f32_type.const_float(1.0).into());
                    }
                    Instruction::Fconst2 => {
                        state.push_value(ctx.f32_type.const_float(2.0).into());
                    }
                    Instruction::Lconst0 => {
                        state.push_value(ctx.i64_type.const_zero().into());
                    }
                    Instruction::Lconst1 => {
                        state.push_value(ctx.i64_type.const_int(1, false).into());
                    }
                    Instruction::Dconst0 => {
                        state.push_value(ctx.f64_type.const_zero().into());
                    }
                    Instruction::Dconst1 => {
                        state.push_value(ctx.f64_type.const_float(1.0).into());
                    }
                    Instruction::Bipush(value) => {
                        let byte = ctx.i8_type.const_int(value as u64, true);
                        let extended = ctx.builder.build_int_s_extend(byte, ctx.i32_type, "bipush");
                        state.push_value(extended.into());
                    }
                    Instruction::Sipush(value) => {
                        let short = ctx.i16_type.const_int(value as u64, true);
                        let extended =
                            ctx.builder
                                .build_int_s_extend(short, ctx.i32_type, "sipush");
                        state.push_value(extended.into());
                    }

                    // ---- loads ----
                    Instruction::Iload0
                    | Instruction::Iload1
                    | Instruction::Iload2
                    | Instruction::Iload3
                    | Instruction::Lload0
                    | Instruction::Lload1
                    | Instruction::Lload2
                    | Instruction::Lload3
                    | Instruction::Fload0
                    | Instruction::Fload1
                    | Instruction::Fload2
                    | Instruction::Fload3
                    | Instruction::Dload0
                    | Instruction::Dload1
                    | Instruction::Dload2
                    | Instruction::Dload3 => {
                        let index = match instr {
                            Instruction::Iload0
                            | Instruction::Lload0
                            | Instruction::Fload0
                            | Instruction::Dload0 => 0,
                            Instruction::Iload1
                            | Instruction::Lload1
                            | Instruction::Fload1
                            | Instruction::Dload1 => 1,
                            Instruction::Iload2
                            | Instruction::Lload2
                            | Instruction::Fload2
                            | Instruction::Dload2 => 2,
                            Instruction::Iload3
                            | Instruction::Lload3
                            | Instruction::Fload3
                            | Instruction::Dload3 => 3,
                            _ => unreachable!(),
                        };
                        let v = state.get_local(index);
                        state.push_value(v);
                    }

                    Instruction::Iload(index)
                    | Instruction::Lload(index)
                    | Instruction::Fload(index)
                    | Instruction::Dload(index) => {
                        let v = state.get_local(index as usize);
                        state.push_value(v);
                    }

                    Instruction::Aaload => {
                        // load onto the stack a reference from an array
                        let index = state.pop_value();
                        let array_ref = state.pop_value().into_pointer_value();

                        // First, we need to dereference the array_ref to get the address to the first element of the array.
                        let array_data_ptr_ptr = ctx
                            .builder
                            .build_struct_gep(
                                ctx.java_array_struct_type,
                                array_ref,
                                1,
                                "array_data_ptr_ptr",
                            )
                            .unwrap();
                        let array_data_ptr = ctx.builder.build_load(
                            ctx.void_ptr,
                            array_data_ptr_ptr,
                            "array_data_ptr",
                        );

                        // TODO: out of bounds check.

                        // Next, we need to get the pointer to the element at the index.
                        let element_ptr = unsafe {
                            ctx.builder.build_gep(
                                ctx.ptr_sized_type.ptr_type(AddressSpace::default()),
                                array_data_ptr.into_pointer_value(),
                                &[index.into_int_value()],
                                "element_ptr",
                            )
                        };

                        // Finally, we need to load the element.
                        let element = ctx.builder.build_load(ctx.void_ptr, element_ptr, "element");

                        state.push_value(element);
                    }

                    Instruction::Aload0
                    | Instruction::Aload1
                    | Instruction::Aload2
                    | Instruction::Aload3 => {
                        let index = match instr {
                            Instruction::Aload0 => 0,
                            Instruction::Aload1 => 1,
                            Instruction::Aload2 => 2,
                            Instruction::Aload3 => 3,
                            _ => unreachable!(),
                        };
                        let v = state.locals.get(index).unwrap().clone().unwrap();
                        state.push_value(v);
                    }

                    Instruction::Iaload
                    | Instruction::Baload
                    | Instruction::Saload
                    | Instruction::Caload
                    | Instruction::Laload
                    | Instruction::Faload
                    | Instruction::Daload => {
                        let typ = match instr {
                            Instruction::Iaload
                            | Instruction::Baload
                            | Instruction::Saload
                            | Instruction::Caload => ctx.i32_type.as_basic_type_enum(),
                            Instruction::Laload => ctx.i64_type.as_basic_type_enum(),
                            Instruction::Faload => ctx.f32_type.as_basic_type_enum(),
                            Instruction::Daload => ctx.f64_type.as_basic_type_enum(),
                            _ => unreachable!("{}: {:?}", addr, instr),
                        };

                        let index = state.pop_value();
                        let array_ref = state.pop_value().into_pointer_value();

                        // First, we need to dereference the array_ref to get the address to the first element of the array.
                        let array_data_ptr_ptr = ctx
                            .builder
                            .build_struct_gep(
                                ctx.java_array_struct_type,
                                array_ref,
                                1,
                                "array_data_ptr_ptr",
                            )
                            .unwrap();
                        let array_data_ptr = ctx.builder.build_load(
                            ctx.void_ptr,
                            array_data_ptr_ptr,
                            "array_data_ptr",
                        );

                        let value_ptr = unsafe {
                            ctx.builder.build_gep(
                                ctx.i64_type, // All element has 64 bit slot.
                                array_data_ptr.into_pointer_value(),
                                &[index.into_int_value()],
                                "element_ptr",
                            )
                        };

                        let loaded = ctx.builder.build_load(typ, value_ptr, "loaded");
                        state.push_value(loaded);
                    }

                    // ------------ stores ------------
                    Instruction::Astore0
                    | Instruction::Astore1
                    | Instruction::Astore2
                    | Instruction::Astore3 => {
                        let index = match instr {
                            Instruction::Astore0 => 0,
                            Instruction::Astore1 => 1,
                            Instruction::Astore2 => 2,
                            Instruction::Astore3 => 3,
                            _ => unreachable!(),
                        };
                        let v = state.pop_value();
                        state.set_local(index, v);
                    }

                    Instruction::Istore0
                    | Instruction::Istore1
                    | Instruction::Istore2
                    | Instruction::Istore3
                    | Instruction::Lstore0
                    | Instruction::Lstore1
                    | Instruction::Lstore2
                    | Instruction::Lstore3
                    | Instruction::Fstore0
                    | Instruction::Fstore1
                    | Instruction::Fstore2
                    | Instruction::Fstore3
                    | Instruction::Dstore0
                    | Instruction::Dstore1
                    | Instruction::Dstore2
                    | Instruction::Dstore3 => {
                        let index = match instr {
                            Instruction::Istore0
                            | Instruction::Lstore0
                            | Instruction::Fstore0
                            | Instruction::Dstore0 => 0,
                            Instruction::Istore1
                            | Instruction::Lstore1
                            | Instruction::Fstore1
                            | Instruction::Dstore1 => 1,
                            Instruction::Istore2
                            | Instruction::Lstore2
                            | Instruction::Fstore2
                            | Instruction::Dstore2 => 2,
                            Instruction::Istore3
                            | Instruction::Lstore3
                            | Instruction::Fstore3
                            | Instruction::Dstore3 => 3,
                            _ => unreachable!(),
                        };
                        let v = state.pop_value();
                        state.set_local(index, v);
                    }

                    Instruction::Istore(index)
                    | Instruction::Lstore(index)
                    | Instruction::Fstore(index)
                    | Instruction::Dstore(index) => {
                        let v = state.pop_value();
                        state.set_local(index as usize, v);
                    }

                    Instruction::Iastore
                    | Instruction::Bastore
                    | Instruction::Sastore
                    | Instruction::Castore
                    | Instruction::Lastore
                    | Instruction::Fastore
                    | Instruction::Dastore => {
                        let value = state.pop_value();
                        let index = state.pop_value();
                        let array_ref = state.pop_value().into_pointer_value();

                        // First, we need to dereference the array_ref to get the address to the first element of the array.
                        let array_data_ptr_ptr = ctx
                            .builder
                            .build_struct_gep(
                                ctx.java_array_struct_type,
                                array_ref,
                                1,
                                "array_data_ptr_ptr",
                            )
                            .unwrap();
                        let array_data_ptr = ctx.builder.build_load(
                            ctx.void_ptr,
                            array_data_ptr_ptr,
                            "array_data_ptr",
                        );

                        let value_ptr = unsafe {
                            ctx.builder.build_gep(
                                ctx.i64_type, // All element has 64 bit slot.
                                array_data_ptr.into_pointer_value(),
                                &[index.into_int_value()],
                                "element_ptr",
                            )
                        };

                        ctx.builder.build_store(value_ptr, value);
                    }

                    // ------- function calls --------
                    Instruction::Invokevirtual(index) => {
                        let method_ref = match self.get_const(index as usize) {
                            ConstantInfo::MethodRef(method_ref) => method_ref,
                            v => unreachable!("{:?}", v),
                        };

                        let (class_name, method_name, descriptor) = self.resolve_class_field(
                            method_ref.class_index as usize,
                            method_ref.name_and_type_index as usize,
                        );

                        let fn_type = {
                            let method_type = descriptor::parse_method_descriptor(&descriptor);
                            ctx.llvm_function_type_from_method_type(
                                &method_type,
                                false, /* virtual invocation == non static */
                            )
                        };

                        let mut args = Vec::new();
                        for _ in 0..fn_type.count_param_types() - 2 {
                            // -2 because the first parameter is the runtime object pointer,
                            // and the second parameter is the object pointer. They will be pushed later.
                            args.push(state.pop_value().into());
                        }

                        // Get the runtime context pointer from the first parameter of this function.
                        let rt_ctx_ptr = state
                            .function()
                            .get_nth_param(0)
                            .unwrap()
                            .into_pointer_value();

                        let obj_ptr = state.pop_value().into_pointer_value();
                        // Next, we need to load the vtable pointer from the object which is the first field.
                        // The pointer we deal with is *i8, so simply dereference it.
                        let vtable_ptr =
                            ctx.builder.build_load(ctx.void_ptr, obj_ptr, "vtable_ptr");

                        // Get the offset in the vtable for the target method.
                        let symbol = format!("{}.{}:{}", class_name, method_name, descriptor);
                        let vtable_offset = ctx.get_virtual_method_offset_value(&symbol);

                        let func_ptr_ptr = unsafe {
                            ctx.builder.build_gep(
                                fn_type.ptr_type(AddressSpace::default()),
                                vtable_ptr.into_pointer_value(),
                                &[vtable_offset.into()],
                                "func_ptr_ptr",
                            )
                        };

                        // vtable exists at the first field of any object.
                        let func_ptr = ctx
                            .builder
                            .build_load(ctx.void_ptr, func_ptr_ptr, symbol.as_str())
                            .into_pointer_value();

                        args.push(obj_ptr.into());
                        args.push(rt_ctx_ptr.into()); // First argument is always rt_ctx_ptr.
                        args.reverse();

                        let ret_val = ctx
                            .builder
                            .build_indirect_call(fn_type, func_ptr, &args, "call")
                            .try_as_basic_value()
                            .left();

                        if let Some(ret) = ret_val {
                            state.push_value(ret);
                        }
                    }

                    Instruction::Invokestatic(index) => {
                        let method_ref = match self.get_const(index as usize) {
                            ConstantInfo::MethodRef(method_ref) => method_ref,
                            v => unreachable!("{:?}", v),
                        };

                        let (class_name, method_name, descriptor_str) = self.resolve_class_field(
                            method_ref.class_index as usize,
                            method_ref.name_and_type_index as usize,
                        );

                        let descriptor = parse_method_descriptor(&descriptor_str);

                        if !self.class_name.clone().eq(&class_name) {
                            panic!("TODO: non-local static invocation: {}", class_name)
                        } else {
                            let (method, _) = self.get_local_method_by_symbol(
                                ctx,
                                &method_name,
                                &descriptor_str,
                                &descriptor,
                                true, // static invocation.
                            );

                            let mut args = Vec::new();
                            for _ in 0..method.count_params() - 1 {
                                // -1 because the first parameter is the runtime object pointer.
                                args.push(state.pop_value().into());
                            }

                            // Get the runtime context pointer from the first parameter of this function.
                            let rt_ctx_ptr = state
                                .function()
                                .get_nth_param(0)
                                .unwrap()
                                .into_pointer_value();

                            args.push(rt_ctx_ptr.into()); // First argument is always rt_ctx_ptr.
                            args.reverse();

                            let ret_val = ctx
                                .builder
                                .build_call(method, &args, "call")
                                .try_as_basic_value()
                                .left();

                            if let Some(ret) = ret_val {
                                state.push_value(ret);
                            }
                        }
                    }

                    Instruction::Invokespecial(index) => {
                        let method_ref = match self.get_const(index as usize) {
                            ConstantInfo::MethodRef(method_ref) => method_ref,
                            v => unreachable!("{:?}", v),
                        };

                        let (_, _, _) = self.resolve_class_field(
                            method_ref.class_index as usize,
                            method_ref.name_and_type_index as usize,
                        );
                    }

                    Instruction::Ldc(index) => self.load_constant(ctx, state, index as u16),
                    Instruction::Ldc2W(index) => self.load_constant(ctx, state, index),

                    Instruction::Getstatic(index) => {
                        let field_ref = match self.get_const(index as usize) {
                            ConstantInfo::FieldRef(method_ref) => method_ref,
                            v => unreachable!("{:?}", v),
                        };

                        let (class_name, field_name, descriptor) = self.resolve_class_field(
                            field_ref.class_index as usize,
                            field_ref.name_and_type_index as usize,
                        );

                        let typ = match parse_field_type_descriptor(&descriptor) {
                            FieldType::ObjectType(_) => ctx.void_ptr.into(),
                            FieldType::BaseType(BaseType::Boolean)
                            | FieldType::BaseType(BaseType::Byte)
                            | FieldType::BaseType(BaseType::Short)
                            | FieldType::BaseType(BaseType::Char)
                            | FieldType::BaseType(BaseType::Int) => ctx.i32_type.into(),
                            FieldType::BaseType(BaseType::Long) => ctx.i64_type.into(),
                            FieldType::BaseType(BaseType::Float) => ctx.f32_type.into(),
                            FieldType::BaseType(BaseType::Double) => ctx.f64_type.into(),
                            field_type => unreachable!("{:?}", field_type),
                        };

                        let field_ptr = load_class_obj_static_field_ptr(
                            ctx,
                            &class_name,
                            state.isolate_ptr(),
                            &field_name,
                            typ,
                        );
                        let loaded = ctx.builder.build_load(typ, field_ptr, &field_name);
                        state.push_value(loaded.into());
                    }

                    Instruction::Putstatic(index) => {
                        let field_ref = match self.get_const(index as usize) {
                            ConstantInfo::FieldRef(field_ref) => field_ref,
                            v => unreachable!("{:?}", v),
                        };
                        let (class_name, field_name, _) = self.resolve_class_field(
                            field_ref.class_index as usize,
                            field_ref.name_and_type_index as usize,
                        );

                        let value = state.pop_value();

                        let field_ptr = load_class_obj_static_field_ptr(
                            ctx,
                            &class_name,
                            state.isolate_ptr(),
                            &field_name,
                            value.get_type(),
                        );
                        ctx.builder.build_store(field_ptr, value);
                    }

                    Instruction::Arraylength => {
                        let array_ref = state.pop_value().into_pointer_value();
                        let length_ptr = ctx
                            .builder
                            .build_struct_gep(
                                ctx.java_array_struct_type,
                                array_ref,
                                2,
                                "length_ptr",
                            )
                            .unwrap();

                        let length_value =
                            ctx.builder.build_load(ctx.i32_type, length_ptr, "length");
                        state.push_value(length_value);
                    }
                    Instruction::Iadd | Instruction::Ladd => {
                        let v2 = state.pop_value();
                        let v1 = state.pop_value();
                        let result = ctx.builder.build_int_add(
                            v1.into_int_value(),
                            v2.into_int_value(),
                            "result",
                        );
                        state.push_value(result.into());
                    }
                    Instruction::Isub | Instruction::Lsub => {
                        let v2 = state.pop_value();
                        let v1 = state.pop_value();
                        let result = ctx.builder.build_int_sub(
                            v1.into_int_value(),
                            v2.into_int_value(),
                            "result",
                        );
                        state.push_value(result.into());
                    }
                    Instruction::Idiv | Instruction::Ldiv => {
                        // TODO: exception!
                        let v2 = state.pop_value();
                        let v1 = state.pop_value();
                        let result = ctx.builder.build_int_signed_div(
                            v1.into_int_value(),
                            v2.into_int_value(),
                            "result",
                        );
                        state.push_value(result.into());
                    }
                    Instruction::Imul | Instruction::Lmul => {
                        let v2 = state.pop_value();
                        let v1 = state.pop_value();
                        let result = ctx.builder.build_int_mul(
                            v1.into_int_value(),
                            v2.into_int_value(),
                            "result",
                        );
                        state.push_value(result.into());
                    }

                    Instruction::Fadd | Instruction::Dadd => {
                        let v2 = state.pop_value();
                        let v1 = state.pop_value();
                        let result = ctx.builder.build_float_add(
                            v1.into_float_value(),
                            v2.into_float_value(),
                            "result",
                        );
                        state.push_value(result.into());
                    }

                    Instruction::Fsub | Instruction::Dsub => {
                        let v2 = state.pop_value();
                        let v1 = state.pop_value();
                        let result = ctx.builder.build_float_sub(
                            v1.into_float_value(),
                            v2.into_float_value(),
                            "result",
                        );
                        state.push_value(result.into());
                    }

                    Instruction::Fmul | Instruction::Dmul => {
                        let v2 = state.pop_value();
                        let v1 = state.pop_value();
                        let result = ctx.builder.build_float_mul(
                            v1.into_float_value(),
                            v2.into_float_value(),
                            "result",
                        );
                        state.push_value(result.into());
                    }

                    Instruction::Fdiv | Instruction::Ddiv => {
                        let v2 = state.pop_value();
                        let v1 = state.pop_value();
                        let result = ctx.builder.build_float_div(
                            v1.into_float_value(),
                            v2.into_float_value(),
                            "result",
                        );
                        state.push_value(result.into());
                    }

                    Instruction::I2b => {
                        let v = state.pop_value();
                        let result_8 = ctx.builder.build_int_truncate(
                            v.into_int_value(),
                            ctx.i8_type,
                            "result",
                        );
                        let result =
                            ctx.builder
                                .build_int_s_extend(result_8, ctx.i32_type, "result");
                        state.push_value(result.into());
                    }
                    Instruction::I2s => {
                        let v = state.pop_value();
                        let result_16 = ctx.builder.build_int_truncate(
                            v.into_int_value(),
                            ctx.i16_type,
                            "result_16",
                        );
                        let result =
                            ctx.builder
                                .build_int_s_extend(result_16, ctx.i32_type, "result");
                        state.push_value(result.into());
                    }
                    Instruction::I2c => {
                        let v = state.pop_value();
                        let result_c = ctx.builder.build_int_truncate(
                            v.into_int_value(),
                            ctx.i16_type,
                            "result",
                        );
                        let result =
                            ctx.builder
                                .build_int_z_extend(result_c, ctx.i32_type, "result");
                        state.push_value(result.into());
                    }

                    Instruction::Iinc { index, value } => {
                        let value = value as i64; // sign extend
                        let local = state.get_local(index as usize);
                        let new_local = ctx.builder.build_int_add(
                            local.into_int_value(),
                            local
                                .get_type()
                                .into_int_type()
                                .const_int(value as u64, true),
                            "new_local",
                        );
                        state.set_local(index as usize, new_local.into());
                    }

                    // ---------- control flows ------------
                    Instruction::IfIcmpge(diff)
                    | Instruction::IfIcmpgt(diff)
                    | Instruction::IfIcmple(diff)
                    | Instruction::IfIcmplt(diff)
                    | Instruction::IfIcmpne(diff)
                    | Instruction::IfIcmpeq(diff) => {
                        let then_label = self.label_with_offset(state, addr, diff);
                        state.set_locals_edges(ctx, then_label);
                        let else_label = self.label_with_offset(state, addr, 3);
                        state.set_locals_edges(ctx, else_label);

                        let v2 = state.pop_value();
                        let v1 = state.pop_value();
                        let cond = ctx.builder.build_int_compare(
                            {
                                match instr {
                                    Instruction::IfIcmpge(_) => IntPredicate::SGE,
                                    Instruction::IfIcmpgt(_) => IntPredicate::SGT,
                                    Instruction::IfIcmple(_) => IntPredicate::SLE,
                                    Instruction::IfIcmplt(_) => IntPredicate::SLT,
                                    Instruction::IfIcmpne(_) => IntPredicate::NE,
                                    Instruction::IfIcmpeq(_) => IntPredicate::EQ,
                                    _ => unreachable!(),
                                }
                            },
                            v1.into_int_value(),
                            v2.into_int_value(),
                            "cond",
                        );
                        ctx.builder
                            .build_conditional_branch(cond, then_label, else_label);
                        terminated = true;
                    }

                    Instruction::Ifne(diff)
                    | Instruction::Ifeq(diff)
                    | Instruction::Ifgt(diff)
                    | Instruction::Ifge(diff)
                    | Instruction::Ifle(diff) => {
                        let then_label = self.label_with_offset(state, addr, diff);
                        state.set_locals_edges(ctx, then_label);
                        let else_label = self.label_with_offset(state, addr, 3);
                        state.set_locals_edges(ctx, else_label);

                        let v = state.pop_value();
                        let cond = ctx.builder.build_int_compare(
                            {
                                match instr {
                                    Instruction::Ifne(_) => IntPredicate::NE,
                                    Instruction::Ifeq(_) => IntPredicate::EQ,
                                    Instruction::Ifgt(_) => IntPredicate::SGT,
                                    Instruction::Ifge(_) => IntPredicate::SGE,
                                    Instruction::Ifle(_) => IntPredicate::SLE,
                                    _ => unreachable!(),
                                }
                            },
                            v.into_int_value(),
                            v.get_type().const_zero().into_int_value(),
                            "cond",
                        );
                        ctx.builder
                            .build_conditional_branch(cond, then_label, else_label);
                        terminated = true;
                    }

                    Instruction::Lcmp => {
                        let w = state.pop_value().as_basic_value_enum().into_int_value();
                        let v = state.pop_value().as_basic_value_enum().into_int_value();
                        let is_equal =
                            ctx.builder
                                .build_int_compare(IntPredicate::EQ, v, w, "is_equal");

                        let selected = ctx.builder.build_select(
                            is_equal,
                            ctx.i32_type.const_zero().as_basic_value_enum(),
                            ctx.i32_type
                                .const_int((-1i64) as u64, true)
                                .as_basic_value_enum(),
                            "selected",
                        );

                        let is_greater =
                            ctx.builder
                                .build_int_compare(IntPredicate::SGT, v, w, "is_greater");

                        let selected = ctx.builder.build_select(
                            is_greater,
                            ctx.i32_type.const_int(1, false).as_basic_value_enum(),
                            selected,
                            "selected",
                        );
                        state.push_value(selected);
                    }

                    Instruction::Fcmpl
                    | Instruction::Dcmpl
                    | Instruction::Fcmpg
                    | Instruction::Dcmpg => {
                        let w = state.pop_value().as_basic_value_enum().into_float_value();
                        let v = state.pop_value().as_basic_value_enum().into_float_value();
                        let one = ctx.i32_type.const_int(1, true).as_basic_value_enum();
                        let minus_one = ctx
                            .i32_type
                            .const_int((-1i64) as u64, true)
                            .as_basic_value_enum();
                        let zero = ctx.i32_type.const_zero().as_basic_value_enum();
                        let either_is_nan = ctx.builder.build_float_compare(
                            FloatPredicate::UNO,
                            v,
                            w,
                            "either_is_nan",
                        );
                        let nan_result = {
                            if instr == Instruction::Fcmpl || instr == Instruction::Dcmpl {
                                minus_one
                            } else {
                                one
                            }
                        };

                        let is_greater = ctx.builder.build_float_compare(
                            FloatPredicate::OGT,
                            v,
                            w,
                            "is_greater",
                        );

                        let is_eq =
                            ctx.builder
                                .build_float_compare(FloatPredicate::OEQ, v, w, "is_equal");

                        let result_eq =
                            ctx.builder
                                .build_select(is_eq, zero, minus_one, "result_lt");

                        let result_eq_or_gt =
                            ctx.builder
                                .build_select(is_greater, one, result_eq, "result_eq_or_gt");

                        let result = ctx.builder.build_select(
                            either_is_nan,
                            nan_result,
                            result_eq_or_gt,
                            "result_eq_or_gt",
                        );
                        state.push_value(result);
                    }

                    Instruction::Goto(diff) => {
                        let label = self.label_with_offset(state, addr, diff);
                        state.set_locals_edges(ctx, label);
                        ctx.builder.build_unconditional_branch(label);
                        terminated = true;
                    }

                    Instruction::Areturn => {
                        if self.tracing_enabled {
                            insert_call_tracing_after(ctx, state);
                        }
                        let v = state.pop_value();
                        ctx.builder.build_return(Some(&v));
                        terminated = true;
                    }

                    Instruction::Ireturn => {
                        if self.tracing_enabled {
                            insert_call_tracing_after(ctx, state);
                        }
                        let v = state.pop_value();
                        ctx.builder.build_return(Some(&v));
                        terminated = true;
                    }

                    Instruction::Return => {
                        if self.tracing_enabled {
                            insert_call_tracing_after(ctx, state);
                        }
                        if state.function().get_type().get_return_type().is_some() {
                            ctx.builder.build_return(Some(&state.pop_value()));
                        } else {
                            ctx.builder.build_return(None);
                        }
                        terminated = true;
                    }
                    instr => {
                        panic!("{:?}", instr);
                    }
                }
            }
        }
    }
}
