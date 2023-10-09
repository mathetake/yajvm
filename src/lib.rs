pub mod codegen;
pub mod stdlib;

use inkwell::execution_engine::JitFunction;
use std::io::Write;

mod compiled_class;
pub mod isolate;
pub mod tracing;

pub use crate::codegen::CodeGen;
pub use crate::isolate::Isolate;
use crate::stdlib::add_stdlib;

pub enum StdoutOption {
    Stdout(Box<dyn Stdout>),
    VecOutputStream,
    HostStdout,
}

/// The trait that represents the standard output stream.
pub trait Stdout {
    fn write(&mut self, b: &[u8], off: i32, len: i32);
    fn buffer(&self) -> &Vec<u8>;
}

impl Stdout for std::io::Stdout {
    fn write(&mut self, b: &[u8], off: i32, len: i32) {
        let b = &b[off as usize..(off + len) as usize];
        self.write_all(b).unwrap();
    }

    fn buffer(&self) -> &Vec<u8> {
        panic!("Not implemented");
    }
}

pub struct VecOutputStream {
    pub buf: Vec<u8>,
}

impl VecOutputStream {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }
}

impl Stdout for VecOutputStream {
    fn write(&mut self, b: &[u8], off: i32, len: i32) {
        let b = &b[off as usize..(off + len) as usize];
        self.buf.extend_from_slice(b);
    }

    fn buffer(&self) -> &Vec<u8> {
        &self.buf
    }
}

pub struct JitEnv<'ctx> {
    codegen: CodeGen<'ctx>,
}

impl<'ctx> JitEnv<'ctx> {
    pub fn new(class_name: &str) -> Self {
        let mut codegen = CodeGen::new(class_name);
        add_stdlib(&mut codegen);
        Self { codegen }
    }

    pub fn new_isolate(&self, stdout: StdoutOption) -> Isolate {
        let stdout = match stdout {
            StdoutOption::Stdout(stdout) => stdout,
            StdoutOption::VecOutputStream => Box::new(VecOutputStream::new()),
            StdoutOption::HostStdout => Box::new(std::io::stdout()),
        };
        let isolate = Isolate::new(&self.codegen, stdout);
        isolate
    }

    pub fn compile(&mut self, path: &str) {
        self.codegen.compile(path);
    }

    pub fn dump_llvm_module(&mut self, path: &str) {
        self.codegen.dump_llvm_module(path);
        // self.codegen.cc.module.verify().unwrap();
    }

    pub fn enable_tracing(&mut self) {
        self.codegen.enable_tracing();
    }

    pub fn done_compilation(&mut self) {
        self.codegen.done_compilation();
        if let Some(f) = self
            .codegen
            .cc
            .module
            .get_function("___yajvm_tracing_before")
        {
            self.codegen
                .cc
                .execution_engine
                .add_global_mapping(&f, tracing::before as usize);
        }
        if let Some(f) = self
            .codegen
            .cc
            .module
            .get_function("___yajvm_tracing_after")
        {
            self.codegen
                .cc
                .execution_engine
                .add_global_mapping(&f, tracing::after as usize);
        }

        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.new_class_object_fn,
            Isolate::new_class_object as usize,
        );
        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.get_class_object_fn,
            Isolate::get_class_object as usize,
        );
        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.new_instance_fn,
            Isolate::new_instance as usize,
        );
        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.new_java_array_fn,
            Isolate::new_java_array as usize,
        );
        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.new_boolean_array_fn,
            Isolate::new_bool_java_array as usize,
        );
        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.new_char_array_fn,
            Isolate::new_char_java_array as usize,
        );
        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.new_byte_array_fn,
            Isolate::new_byte_java_array as usize,
        );
        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.new_short_array_fn,
            Isolate::new_short_java_array as usize,
        );
        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.new_int_array_fn,
            Isolate::new_int_java_array as usize,
        );
        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.new_long_array_fn,
            Isolate::new_long_java_array as usize,
        );
        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.new_float_array_fn,
            Isolate::new_float_java_array as usize,
        );
        self.codegen.cc.execution_engine.add_global_mapping(
            &self.codegen.cc.new_double_array_fn,
            Isolate::new_double_java_array as usize,
        );
    }

    pub fn call(&mut self, isolate: &mut Isolate, args: &Vec<String>) {
        let f: JitFunction<'ctx, unsafe extern "C" fn(*mut Isolate, *const Vec<String>)> = unsafe {
            self.codegen
                .cc
                .execution_engine
                .get_function("main")
                .unwrap()
        };

        unsafe {
            f.call(isolate, args);
        }
    }
}
