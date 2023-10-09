use crate::codegen::descriptor::{BaseType, FieldType};
use crate::codegen::CodegenContext;
use crate::codegen::CompilationState;
use crate::tracing::tracing;
use inkwell::module::Linkage::External;
use inkwell::types::BasicType;
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};
use std::cmp::min;

pub fn insert_call_tracing_before<'ctx>(
    ctx: &mut CodegenContext<'ctx>,
    state: &mut CompilationState<'ctx>,
) {
    // This must be synced with the one in src/tracing/tracing.rs.
    let before_fn = ctx
        .module
        .get_function("___yajvm_tracing_before")
        .unwrap_or_else(|| {
            let fn_type = ctx.context.void_type().fn_type(
                &[
                    ctx.void_ptr.into(),
                    ctx.void_ptr.into(),
                    ctx.i32_type.into(),
                ],
                false,
            );
            let function = ctx
                .module
                .add_function("___yajvm_tracing_before", fn_type, None);
            function
        });

    let isolate_ptr = state.isolate_ptr();
    let symbol_ptr = ctx.get_const_string_global(state.function_symbol());

    let params = state.param_count();
    if params > 0 {
        // If there's a parameter to this function, we need to store them into the tracing context.

        let args_vec_ptr = {
            // tracing_ctx_ptr sits at the first field of the RtCtxRaw, and the first field of the tracing context is the fixed-length args_vec.
            ctx.builder
                .build_load(ctx.void_ptr, isolate_ptr.into(), "tracing_ctx_ptr")
                .into_pointer_value()
        };

        let mut local_index = 0;
        for (i, param) in state
            .function()
            .get_params()
            .iter()
            .skip(1) // Skip the *Isolate.
            .enumerate()
            .take(min(tracing::MAX_TRACING_ARGS, params))
        {
            let field_type = state
                .local_field_types()
                .get(local_index)
                .unwrap()
                .as_ref()
                .unwrap();
            store_value_into_tracing_context_slot(
                isolate_ptr,
                args_vec_ptr,
                i,
                field_type,
                ctx,
                *param,
            );

            match field_type {
                FieldType::BaseType(BaseType::Double) | FieldType::BaseType(BaseType::Long) => {
                    local_index += 2;
                }
                _ => local_index += 1,
            }
        }
    }

    let arg_num = ctx.i32_type.const_int(params as u64, false);

    // Call the tracing function.
    ctx.builder.build_call(
        before_fn,
        &[isolate_ptr.into(), symbol_ptr.into(), arg_num.into()],
        "tracing_before",
    );
}

pub fn insert_call_tracing_after<'ctx>(
    ctx: &mut CodegenContext<'ctx>,
    state: &mut CompilationState<'ctx>,
) {
    // This must be synced with the one in src/tracing/tracing.rs.
    let after_fn = ctx
        .module
        .get_function("___yajvm_tracing_after")
        .unwrap_or_else(|| {
            let fn_type = ctx.context.void_type().fn_type(
                &[
                    ctx.void_ptr.into(),
                    ctx.void_ptr.into(),
                    ctx.void_ptr.into(),
                ],
                false,
            );
            let function = ctx
                .module
                .add_function("___yajvm_tracing_after", fn_type, None);
            function
        });

    let rt_ctx = state
        .function()
        .get_nth_param(0)
        .unwrap()
        .into_pointer_value();

    let symbol_ptr = ctx.get_const_string_global(state.function_symbol());

    // Call the tracing function.
    let return_ptr = if let Some(_) = state.function().get_type().get_return_type() {
        let ret = state.value_stack_peek();
        get_arg_or_return_as_pointer(
            state.isolate_ptr(),
            state.function_method_type().return_type.as_ref().unwrap(),
            ctx,
            ret,
        )
    } else {
        ctx.void_ptr.const_zero()
    };

    ctx.builder.build_call(
        after_fn,
        &[rt_ctx.into(), symbol_ptr.into(), return_ptr.into()],
        "tracing_after",
    );
}

fn store_value_into_tracing_context_slot<'ctx>(
    isolate_ptr: PointerValue<'ctx>,
    args_vec_ptr: PointerValue<'ctx>,
    i: usize,
    field_type: &FieldType,
    ctx: &mut CodegenContext<'ctx>,
    arg_or_return: BasicValueEnum<'ctx>,
) {
    let slot = unsafe {
        let offset_const = ctx.i32_type.const_int(i as u64, false);
        ctx.builder.build_gep(
            ctx.void_ptr,
            args_vec_ptr,
            &[offset_const.into()],
            format!("args_vec_ptr[{}]", i).as_str(),
        )
    };
    let ptr = get_arg_or_return_as_pointer(isolate_ptr, field_type, ctx, arg_or_return);
    ctx.builder.build_store(slot, ptr);
}

fn get_arg_or_return_as_pointer<'ctx>(
    isolate_ptr: PointerValue<'ctx>,
    field_type: &FieldType,
    ctx: &mut CodegenContext<'ctx>,
    arg_or_return: BasicValueEnum<'ctx>,
) -> PointerValue<'ctx> {
    match field_type {
        FieldType::ObjectType(_) | FieldType::ArrayType(_) => {
            return arg_or_return.into_pointer_value()
        }
        _ => {}
    };

    let (class_name, constructor_symbol, llvm_level_type, casted_arg_or_ret) = match field_type {
        FieldType::BaseType(BaseType::Byte) | FieldType::ObjectTypeJavaLangByte => (
            "java/lang/Byte",
            "java/lang/Byte.init:(B)V",
            ctx.i32_type.as_basic_type_enum(),
            arg_or_return.as_basic_value_enum(),
        ),
        FieldType::BaseType(BaseType::Char) | FieldType::ObjectTypeJavaLangChar => (
            "java/lang/Char",
            "java/lang/Char.init:(C)V",
            ctx.i32_type.as_basic_type_enum(),
            arg_or_return.as_basic_value_enum(),
        ),
        FieldType::BaseType(BaseType::Double) | FieldType::ObjectTypeJavaLangDouble => (
            "java/lang/Double",
            "java/lang/Double.init:(D)V",
            ctx.f64_type.as_basic_type_enum(),
            arg_or_return.as_basic_value_enum(),
        ),
        FieldType::BaseType(BaseType::Float) | FieldType::ObjectTypeJavaLangFloat => (
            "java/lang/Float",
            "java/lang/Float.init:(F)V",
            ctx.f32_type.as_basic_type_enum(),
            arg_or_return.as_basic_value_enum(),
        ),
        FieldType::BaseType(BaseType::Int) | FieldType::ObjectTypeJavaLangInteger => (
            "java/lang/Integer",
            "java/lang/Integer.init:(I)V",
            ctx.i32_type.as_basic_type_enum(),
            arg_or_return.as_basic_value_enum(),
        ),
        FieldType::BaseType(BaseType::Long) | FieldType::ObjectTypeJavaLangLong => (
            "java/lang/Long",
            "java/lang/Long.init:(J)V",
            ctx.i64_type.as_basic_type_enum(),
            arg_or_return.as_basic_value_enum(),
        ),
        FieldType::BaseType(BaseType::Short) | FieldType::ObjectTypeJavaLangShort => (
            "java/lang/Short",
            "java/lang/Short.init:(S)V",
            ctx.i16_type.as_basic_type_enum(),
            arg_or_return.as_basic_value_enum(),
        ),
        FieldType::BaseType(BaseType::Boolean) | FieldType::ObjectTypeJavaLangBoolean => (
            "java/lang/Boolean",
            "java/lang/Boolean.init:(Z)V",
            ctx.i32_type.as_basic_type_enum(),
            arg_or_return.as_basic_value_enum(),
        ),
        _ => unreachable!(),
    };

    let class_id = ctx.get_class_id_value(&class_name.to_string());
    let ptr = ctx
        .builder
        .build_call(
            ctx.new_instance_fn,
            &[isolate_ptr.into(), class_id.into()],
            "new_instance",
        )
        .try_as_basic_value()
        .left()
        .unwrap()
        .into_pointer_value();

    let constructor = {
        if let Some(f) = ctx.module.get_function(constructor_symbol.as_ref()) {
            f
        } else {
            let typ = ctx
                .void_ptr
                .fn_type(&[ctx.void_ptr.into(), llvm_level_type.into()], false);
            ctx.module
                .add_function(constructor_symbol.as_ref(), typ, Some(External))
        }
    };

    ctx.builder.build_call(
        constructor,
        &[ptr.into(), casted_arg_or_ret.into()],
        constructor_symbol.as_ref(),
    );
    ptr
}
