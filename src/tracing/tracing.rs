use crate::stdlib::java_lang_object::{to_java_string_ref, JavaObjectRef};
use crate::stdlib::java_lang_string::{JavaLangString, JavaLangStringRef};
use crate::Isolate;
use std::cmp::min;
use std::io::Write;

#[no_mangle]
pub unsafe extern "C" fn before(isolate: *mut Isolate, fn_symbol: JavaLangStringRef, arg_num: u32) {
    let tracing_ctx = unsafe { (&mut *isolate).tracer() };
    let fn_symbol = &(*fn_symbol);
    tracing_ctx.before(unsafe { &mut *isolate }, fn_symbol, arg_num);
}

#[no_mangle]
pub unsafe extern "C" fn after(
    isolate: *mut Isolate,
    fn_symbol: JavaLangStringRef,
    ret: JavaObjectRef,
) {
    let tracing_ctx = unsafe { (&mut *isolate).tracer() };
    let fn_symbol = &(*fn_symbol);
    tracing_ctx.after(unsafe { &mut *isolate }, fn_symbol, ret);
}

pub const MAX_TRACING_ARGS: usize = 20;

#[repr(C)]
pub struct Tracer {
    args_result_vec: [JavaObjectRef; MAX_TRACING_ARGS],
    depth: usize,
    buf: Vec<u8>,
}

impl Tracer {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            depth: 0,
            args_result_vec: [std::ptr::null_mut(); 20],
        }
    }

    pub fn buffer(&self) -> &Vec<u8> {
        &self.buf
    }

    pub fn before(&mut self, isolate: &mut Isolate, fn_symbol: &JavaLangString, arg_num: u32) {
        // Repeat the character "-" for each level of depth.
        let depth = self.depth;
        for _ in 0..depth {
            self.buf.write_all(b"\t").unwrap();
        }
        self.buf.write_all(b"--> \"").unwrap();

        // Then print the function name.
        self.buf.write_all(fn_symbol.as_bytes()).unwrap();
        self.buf.write_all(b"\" (").unwrap();

        let c = min(arg_num, 20);
        for i in 0..c {
            let obj_ref = self.args_result_vec[i as usize];
            let java_str = to_java_string_ref(isolate, obj_ref);
            self.buf
                .write_all(unsafe { (*java_str).as_bytes() })
                .unwrap();
            if i != c - 1 {
                self.buf.write_all(b", ").unwrap();
            }
        }
        if arg_num > 21 {
            self.buf.write_all(b"...").unwrap();
        }
        self.buf.write_all(b")\n").unwrap();
        self.depth += 1;
    }

    pub fn after(
        &mut self,
        isolate: &mut Isolate,
        _fn_symbol: &JavaLangString,
        ret: JavaObjectRef,
    ) {
        self.depth -= 1;
        // Repeat the character "-" for each level of depth.
        let depth = self.depth;
        for _ in 0..depth {
            self.buf.write_all(b"\t").unwrap();
        }
        self.buf.write_all(b"<-- ").unwrap();
        if ret.is_null() {
            self.buf.write_all(b"void").unwrap();
        } else {
            let java_str = to_java_string_ref(isolate, ret);
            self.buf
                .write_all(unsafe { (*java_str).as_bytes() })
                .unwrap();
        }
        self.buf.write_all(b"\n").unwrap();
    }
}
