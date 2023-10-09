use crate::compiled_class::{CompiledClass, VirtualMethodInfo};
use crate::stdlib::java_lang_object::JavaObjectRef;
use crate::stdlib::java_lang_string::JavaLangString;
use crate::Isolate;

pub fn new_compiled_class_java_io_print_stream() -> CompiledClass {
    let mut c = CompiledClass::new("java/io/PrintStream", None);
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/io/PrintStream.println:(Ljava/lang/String;)V".to_string(),
        ptr: Some(PrintStream::println_string as *const u8),
        overrides: None,
    });
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/io/PrintStream.println:(B)V".to_string(),
        ptr: Some(PrintStream::println_byte as *const u8),
        overrides: None,
    });
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/io/PrintStream.println:(C)V".to_string(),
        ptr: Some(PrintStream::println_char as *const u8),
        overrides: None,
    });
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/io/PrintStream.println:(S)V".to_string(),
        ptr: Some(PrintStream::println_short as *const u8),
        overrides: None,
    });
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/io/PrintStream.println:(D)V".to_string(),
        ptr: Some(PrintStream::println_double as *const u8),
        overrides: None,
    });
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/io/PrintStream.println:(F)V".to_string(),
        ptr: Some(PrintStream::println_float as *const u8),
        overrides: None,
    });
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/io/PrintStream.println:(I)V".to_string(),
        ptr: Some(PrintStream::println_int as *const u8),
        overrides: None,
    });
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/io/PrintStream.println:(J)V".to_string(),
        ptr: Some(PrintStream::println_long as *const u8),
        overrides: None,
    });
    c.virtual_methods.push(VirtualMethodInfo {
        symbol: "java/io/PrintStream.println:(Z)V".to_string(),
        ptr: Some(PrintStream::println_boolean as *const u8),
        overrides: None,
    });
    c.instance_size = std::mem::size_of::<PrintStream>() as u32;
    c
}

#[repr(C)]
/// PrintStream corresponds to java/io/PrintStream class.
/// https://docs.oracle.com/javase/jp/8/docs/api/java/io/PrintStream.html
pub struct PrintStream {
    vtable: *const u8,
    out: OutPutStreamRef,
}

pub type PrintStreamRef = *mut PrintStream;

impl PrintStream {
    pub fn init_output_stream(
        _isolate: &mut Isolate,
        print_stream: &mut PrintStream,
        out: OutPutStreamRef,
    ) {
        print_stream.out = out;
    }

    pub fn destroy(raw: JavaObjectRef) {
        let print_stream = unsafe { &mut *(raw as *mut PrintStream) };
        unsafe {
            _ = Box::from_raw(print_stream.out);
        }
    }

    /// Implements `println:(Ljava/lang/String;)V`
    pub unsafe extern "C" fn println_string(
        isolate: &mut Isolate,
        print_stream: *mut PrintStream,
        s: *mut JavaLangString,
    ) {
        (*print_stream)._println_string(isolate, &*s);
    }

    pub fn _println_string(&mut self, isolate: &mut Isolate, s: &JavaLangString) {
        unsafe {
            (*self.out).write(isolate, s.as_bytes(), 0, s.len() as i32);
            (*self.out).write(isolate, b"\n", 0, 1)
        };
    }

    pub extern "C" fn println_int(isolate: &mut Isolate, print_stream: *mut PrintStream, i: i32) {
        let s = i.to_string();
        let out = unsafe { *&mut (*print_stream).out };
        unsafe {
            (*out).write(isolate, s.as_bytes(), 0, s.len() as i32);
            (*out).write(isolate, b"\n", 0, 1)
        };
    }

    /// Implements `println:(L)V`
    pub extern "C" fn println_long(isolate: &mut Isolate, print_stream: *mut PrintStream, i: i64) {
        let s = i.to_string();
        let out = unsafe { *&mut (*print_stream).out };
        unsafe {
            (*out).write(isolate, s.as_bytes(), 0, s.len() as i32);
            (*out).write(isolate, b"\n", 0, 1)
        };
    }

    /// Implements `println:(S)V`
    pub extern "C" fn println_short(isolate: &mut Isolate, print_stream: *mut PrintStream, i: u32) {
        let s = (i as i16).to_string();
        let out = unsafe { *&mut (*print_stream).out };
        unsafe {
            (*out).write(isolate, s.as_bytes(), 0, s.len() as i32);
            (*out).write(isolate, b"\n", 0, 1)
        };
    }

    /// Implements `println:(C)V`
    pub extern "C" fn println_char(isolate: &mut Isolate, print_stream: *mut PrintStream, c: u32) {
        let c = char::from_u32(c).unwrap();
        let s = c.to_string();
        let out = unsafe { *&mut (*print_stream).out };
        unsafe {
            (*out).write(isolate, s.as_bytes(), 0, s.len() as i32);
            (*out).write(isolate, b"\n", 0, 1)
        };
    }

    /// Implements `println:(B)V`
    pub extern "C" fn println_byte(isolate: &mut Isolate, print_stream: *mut PrintStream, i: u32) {
        let s = (i as u8).to_string();
        let out = unsafe { *&mut (*print_stream).out };
        unsafe {
            (*out).write(isolate, s.as_bytes(), 0, s.len() as i32);
            (*out).write(isolate, b"\n", 0, 1)
        };
    }

    /// Implements `println:(F)V`
    pub extern "C" fn println_float(isolate: &mut Isolate, print_stream: *mut PrintStream, f: f32) {
        let s = format_float_java_style(f);
        let out = unsafe { *&mut (*print_stream).out };
        unsafe {
            (*out).write(isolate, s.as_bytes(), 0, s.len() as i32);
            (*out).write(isolate, b"\n", 0, 1)
        };
    }

    /// Implements `println:(D)V`
    pub extern "C" fn println_double(
        isolate: &mut Isolate,
        print_stream: *mut PrintStream,
        d: f64,
    ) {
        let s = format_double_java_style(d);
        let out = unsafe { *&mut (*print_stream).out };
        unsafe {
            (*out).write(isolate, s.as_bytes(), 0, s.len() as i32);
            (*out).write(isolate, b"\n", 0, 1)
        };
    }

    /// Implements `println:(Z)V`
    pub extern "C" fn println_boolean(
        isolate: &mut Isolate,
        print_stream: *mut PrintStream,
        i: u32,
    ) {
        let s = (i == 1).to_string();
        let out = unsafe { *&mut (*print_stream).out };
        unsafe {
            (*out).write(isolate, s.as_bytes(), 0, s.len() as i32);
            (*out).write(isolate, b"\n", 0, 1)
        };
    }
}

/// OutputStream corresponds to java/io/OutputStream class.
/// https://docs.oracle.com/javase/jp/8/docs/api/java/io/OutputStream.html
pub trait OutputStream {
    fn write(&mut self, isolate: &mut Isolate, b: &[u8], off: i32, len: i32);
}

pub type OutPutStreamRef = *mut dyn OutputStream;

fn format_float_java_style(f: f32) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f.is_infinite() {
        return if f.is_sign_positive() {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    if f == 0.0 {
        return if f.is_sign_positive() {
            "0.0".to_string()
        } else {
            "-0.0".to_string()
        };
    }
    let abs = f.abs();
    return if abs >= 1e-3 && abs < 1e7 {
        format!("{}", f)
    } else {
        format!("{:E}", f)
    };
}

fn format_double_java_style(d: f64) -> String {
    if d.is_nan() {
        return "NaN".to_string();
    }
    if d.is_infinite() {
        return if d.is_sign_positive() {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    if d == 0.0 {
        return if d.is_sign_positive() {
            "0.0".to_string()
        } else {
            "-0.0".to_string()
        };
    }

    let abs = d.abs();
    return if abs >= 1e-3 && abs < 1e7 {
        format!("{}", d)
    } else {
        format!("{:E}", d)
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_print_format_double_java_style() {
        assert_eq!(format_double_java_style(f64::MAX), "1.7976931348623157E308");
        assert_eq!(format_double_java_style(4.940660e-324), "5E-324");
    }

    #[test]
    fn test_format_float_java_style() {
        assert_eq!(format_float_java_style(30.0), "30"); // TODO: should be "30.0".
        assert_eq!(format_float_java_style(f32::MAX), "3.4028235E38");
        assert_eq!(format_float_java_style(1.4E-45), "1E-45");
    }
}
