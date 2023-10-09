use crate::codegen::CodegenContext;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{AnyValue, PointerValue};
use inkwell::AddressSpace;

pub fn load_class_obj_static_field_ptr<'ctx>(
    ctx: &mut CodegenContext<'ctx>,
    class_name: &String,
    isolate_ptr: PointerValue<'ctx>,
    field_name: &String,
    field_type: BasicTypeEnum<'ctx>,
) -> PointerValue<'ctx> {
    let class_obj_ptr = load_class_obj_ptr(ctx, class_name, isolate_ptr);

    let field_offset = ctx.get_static_filed_offset_value(&class_name, &field_name);
    let ptr = unsafe {
        let fields_ptr = ctx
            .builder
            .build_load(ctx.void_ptr, class_obj_ptr, "static_fields_ptr")
            .into_pointer_value();

        let ptr = ctx.builder.build_gep(
            ctx.i8_type,
            fields_ptr,
            &[field_offset.into()],
            "static_field_ptr_as_byte_ptr",
        );
        ctx.builder.build_pointer_cast(
            ptr,
            field_type.ptr_type(AddressSpace::default()),
            "static_field_ptr",
        )
    };
    ptr
}

fn load_class_obj_ptr<'ctx>(
    ctx: &mut CodegenContext<'ctx>,
    class_name: &String,
    isolate_ptr: PointerValue<'ctx>,
) -> PointerValue<'ctx> {
    let class_id = ctx.get_class_id_value(&class_name);
    let ptr = ctx
        .builder
        .build_call(
            ctx.get_class_object_fn,
            &[
                isolate_ptr.into(),
                class_id.into(),
                ctx.context.bool_type().const_zero().into(),
            ],
            "get_class_object",
        )
        .as_any_value_enum()
        .into_pointer_value();
    ptr
}
