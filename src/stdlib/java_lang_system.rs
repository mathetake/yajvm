use crate::compiled_class::CompiledClass;
use crate::stdlib::java_io::*;
use crate::Isolate;

pub fn new_compiled_class() -> CompiledClass {
    let mut c = CompiledClass::new("java/lang/System", None);
    c.instance_size = 0;
    c.static_fields.push("out".to_string());
    c.clinit = Some(clinit);
    c
}

pub extern "C" fn clinit(isolate: &mut Isolate) {
    let self_class_id = isolate.class_id("java/lang/System");
    let print_stream_class_id = isolate.class_id("java/io/PrintStream");
    let class_object = Isolate::get_class_object(isolate, self_class_id, false);
    let obj = Isolate::new_instance(isolate, print_stream_class_id);
    let print_stream = unsafe { &mut *(obj as *mut PrintStream) };
    PrintStream::init_output_stream(
        isolate,
        print_stream,
        Box::into_raw(Box::new(IsolateStdoutAsJavaIoOutStream)),
    );
    unsafe { (*class_object).set_object(0, obj) };
}

pub struct IsolateStdoutAsJavaIoOutStream;

impl OutputStream for IsolateStdoutAsJavaIoOutStream {
    fn write(&mut self, isolate: &mut Isolate, b: &[u8], off: i32, len: i32) {
        let stdout = isolate.stdout();
        stdout.write(b, off, len);
    }
}
