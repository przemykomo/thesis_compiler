use std::{collections::HashMap, ffi::CString};

use llvm_sys::{
    core::*,
    prelude::*,
    target::{LLVMABISizeOfType, LLVMOffsetOfElement, LLVMTargetDataRef},
};

use crate::{
    ast::{FieldDecl, StructDecl, Type},
    csa::{TypedExpr, TypedFn},
    lower::{self, Variable},
};

#[derive(Debug)]
pub struct FnABIInfo {
    ret: ABIArgInfo,
    args: Vec<ABIArgInfo>,
    llvm_ty: LLVMTypeRef,
    attributes: Vec<(LLVMAttributeRef, u32)>,
}

struct Registers {
    int_registers: i32,
    ssa_registers: i32,
}

#[derive(Debug)]
struct ABIArgInfo {
    pieces: [Piece; 2],
    single_piece: bool,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
struct Piece {
    pub class: X64Class,
    pub meaningful_bits: u8,
    pub vec: bool,
}

impl Piece {
    pub fn new(class: X64Class) -> Piece {
        Piece {
            class,
            meaningful_bits: 0,
            vec: false,
        }
    }
}

#[allow(nonstandard_style)]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum X64Class {
    CLASS_NO_CLASS,
    CLASS_MEMORY,
    CLASS_INTEGER,
    CLASS_SSE,
}

fn create_abi_info(
    args: &[FieldDecl],
    variadic: bool,
    return_ty: &Type,
    context: LLVMContextRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
) -> FnABIInfo {
    unsafe {
        let mut available_registers = Registers {
            int_registers: 6,
            ssa_registers: 8,
        };
        let ret_abi_info = classify(context, source, structs, data_layout, return_ty);
        if let Piece {
            class: X64Class::CLASS_MEMORY,
            ..
        } = ret_abi_info.pieces[0]
        {
            // If the type has class MEMORY, then the caller provides space for the return value
            // and passes the address of this storage in %rdi as if it were the first argument to the
            // function.
            available_registers.int_registers -= 1;
        }

        let args_abi_info: Vec<ABIArgInfo> = args
            .iter()
            .map(|arg| {
                classify_parameter(
                    context,
                    source,
                    structs,
                    data_layout,
                    &arg.ty,
                    &mut available_registers,
                )
            })
            .collect();

        let mut params: Vec<LLVMTypeRef> = Vec::new();
        let mut attributes: Vec<(LLVMAttributeRef, u32)> = Vec::new();
        let retval = if return_ty.is_scalar() {
            lower::to_llvm_type(context, return_ty, source, structs)
        } else {
            if ret_abi_info.single_piece {
                let piece = ret_abi_info.pieces[0];
                match piece.class {
                    X64Class::CLASS_NO_CLASS => LLVMVoidTypeInContext(context),
                    X64Class::CLASS_MEMORY => {
                        //Add ptr as a first param
                        params.push(LLVMPointerTypeInContext(context, 0));
                        let attr = LLVMCreateTypeAttribute(
                            context,
                            LLVMGetEnumAttributeKindForName(c"sret".as_ptr(), 4),
                            lower::to_llvm_type(context, return_ty, source, structs),
                        );
                        // Params are 1 indexed
                        attributes.push((attr, 1));
                        LLVMVoidTypeInContext(context)
                    }
                    X64Class::CLASS_INTEGER => LLVMInt64TypeInContext(context),
                    X64Class::CLASS_SSE => {
                        if piece.vec {
                            let f = LLVMFloatTypeInContext(context);
                            LLVMVectorType(f, 2)
                        } else if piece.meaningful_bits == 32 {
                            LLVMFloatTypeInContext(context)
                        } else {
                            LLVMDoubleTypeInContext(context)
                        }
                    }
                }
            } else {
                let mut members: Vec<LLVMTypeRef> = Vec::new();

                for piece in ret_abi_info.pieces {
                    match piece.class {
                        X64Class::CLASS_NO_CLASS | X64Class::CLASS_MEMORY => unreachable!(),
                        X64Class::CLASS_INTEGER => {
                            members.push(LLVMInt64TypeInContext(context));
                        }
                        X64Class::CLASS_SSE => {
                            let m = if piece.vec {
                                let f = LLVMFloatTypeInContext(context);
                                LLVMVectorType(f, 2)
                            } else if piece.meaningful_bits == 32 {
                                LLVMFloatTypeInContext(context)
                            } else {
                                LLVMDoubleTypeInContext(context)
                            };
                            members.push(m);
                        }
                    }
                }
                LLVMStructTypeInContext(context, members.as_mut_ptr(), members.len() as u32, 0)
            }
        };

        for (abi, arg) in args_abi_info.iter().zip(args) {
            if arg.ty.is_scalar() {
                params.push(lower::to_llvm_type(context, &arg.ty, source, structs));
                continue;
            }
            if abi.single_piece {
                let piece = abi.pieces[0];
                match piece.class {
                    X64Class::CLASS_NO_CLASS => {}
                    X64Class::CLASS_MEMORY => {
                        params.push(LLVMPointerTypeInContext(context, 0));
                        let attr = LLVMCreateTypeAttribute(
                            context,
                            LLVMGetEnumAttributeKindForName(c"byval".as_ptr(), 5),
                            lower::to_llvm_type(context, &arg.ty, source, structs),
                        );
                        // Params are 1 indexed
                        attributes.push((attr, params.len() as u32));
                    }
                    X64Class::CLASS_INTEGER => params.push(LLVMInt64TypeInContext(context)),
                    X64Class::CLASS_SSE => {
                        let p = if piece.vec {
                            let f = LLVMFloatTypeInContext(context);
                            LLVMVectorType(f, 2)
                        } else if piece.meaningful_bits == 32 {
                            LLVMFloatTypeInContext(context)
                        } else {
                            LLVMDoubleTypeInContext(context)
                        };
                        params.push(p);
                    }
                }
            } else {
                for piece in abi.pieces {
                    match piece.class {
                        X64Class::CLASS_NO_CLASS | X64Class::CLASS_MEMORY => unreachable!(),
                        X64Class::CLASS_INTEGER => {
                            params.push(LLVMInt64TypeInContext(context));
                        }
                        X64Class::CLASS_SSE => {
                            let m = if piece.vec {
                                let f = LLVMFloatTypeInContext(context);
                                LLVMVectorType(f, 2)
                            } else if piece.meaningful_bits == 32 {
                                LLVMFloatTypeInContext(context)
                            } else {
                                LLVMDoubleTypeInContext(context)
                            };
                            params.push(m);
                        }
                    }
                }
            }
        }

        let fn_type = LLVMFunctionType(
            retval,
            params.as_mut_ptr(),
            params.len() as u32,
            variadic as i32,
        );

        FnABIInfo {
            ret: ret_abi_info,
            args: args_abi_info,
            llvm_ty: fn_type,
            attributes,
        }
    }
}

fn classify_parameter(
    context: LLVMContextRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
    ty: &Type,
    available_registers: &mut Registers,
) -> ABIArgInfo {
    let mut ret = classify(context, source, structs, data_layout, ty);

    let mut needed_regs = Registers {
        int_registers: 0,
        ssa_registers: 0,
    };

    for piece in ret.pieces {
        match piece.class {
            X64Class::CLASS_MEMORY => break,
            X64Class::CLASS_INTEGER => {
                needed_regs.int_registers += 1;
            }
            X64Class::CLASS_SSE => {
                needed_regs.ssa_registers += 1;
            }
            _ => {}
        }
    }

    // If there are no registers available for any eightbyte of an argument, the whole argument is passed on the stack.
    if available_registers.int_registers < needed_regs.int_registers
        || available_registers.ssa_registers < needed_regs.ssa_registers
    {
        ret.pieces[0].class = X64Class::CLASS_MEMORY;
        ret.single_piece = true;
        return ret;
    }

    available_registers.int_registers -= needed_regs.int_registers;
    available_registers.ssa_registers -= needed_regs.ssa_registers;

    ret
}

fn classify(
    context: LLVMContextRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
    ty: &Type,
) -> ABIArgInfo {
    let mut pieces = [Piece::new(X64Class::CLASS_NO_CLASS); 2];
    let num = classify_merge(context, source, structs, data_layout, ty, &mut pieces, 0);
    if num == 0 {
        pieces[0].class = X64Class::CLASS_MEMORY;
    }

    ABIArgInfo {
        pieces,
        single_piece: num < 2,
    }
}

// Returns the number of words, or 0 if the parameter should be passed in memory.
// As a special case for zero sized containers, classes[0] will be NO_CLASS and 1 is returned.
// byte_offset is an offset within the eightbyte word
fn classify_merge(
    context: LLVMContextRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
    ty: &Type,
    pieces: &mut [Piece; 2],
    byte_offset: usize,
) -> usize {
    use Type::*;
    match ty {
        UnsizedInt | UnsizedFloat => unreachable!(),
        Void => 1,
        I8
        | U8
        | I16
        | U16
        | I32
        | U32
        | I64
        | U64
        | Pointer(_)
        | Char
        | Boolean
        | Fn {
            args: _,
            variadic: _,
            return_ty: _,
        } => {
            let bits_size = match ty {
                Boolean | Char | I8 | U8 => 8,
                I16 | U16 => 16,
                I32 | U32 => 32,
                I64
                | U64
                | Pointer(_)
                | Fn {
                    args: _,
                    variadic: _,
                    return_ty: _,
                } => 64,
                _ => unreachable!(),
            };
            pieces[0] = Piece {
                class: X64Class::CLASS_INTEGER,
                meaningful_bits: byte_offset as u8 * 8 + bits_size,
                vec: false,
            };
            1
        }
        F32 => {
            pieces[0] = Piece {
                class: X64Class::CLASS_SSE,
                meaningful_bits: byte_offset as u8 * 8 + 32,
                // Only checked if this word ends up merged as SSE,
                // and 2 floats are then passed as a vec in LLVM IR
                vec: byte_offset == 4,
            };
            1
        }
        F64 => {
            pieces[0] = Piece {
                class: X64Class::CLASS_SSE,
                meaningful_bits: byte_offset as u8 * 8 + 64,
                vec: false,
            };
            1
        }
        Struct(ident) => unsafe {
            // https://github.com/gcc-mirror/gcc/blob/HEAD@%7B2021-05-05%7D/gcc/config/i386/i386.c#L2080
            let llvm_type = lower::to_llvm_type(context, ty, source, structs);
            let bytes = LLVMABISizeOfType(data_layout, llvm_type) as usize;

            // When size > 16 bytes, if the first one isn't
            // X86_64_SSE_CLASS or any other ones aren't
            // X86_64_SSEUP_CLASS, everything should be passed in
            // memory.
            // Since I don't use SSEUP, I always pass such structs by memory.
            if bytes > 16 {
                return 0;
            }

            assert!(bytes > 0, "TODO: zero-sized structs");

            let mut subpieces = [Piece::new(X64Class::CLASS_NO_CLASS); 2];

            const UNITS_PER_WORD: usize = 8;
            let words = (bytes + byte_offset + UNITS_PER_WORD - 1) / UNITS_PER_WORD;

            let s = structs.get(&source[ident.clone()]).unwrap();

            for (i, field) in s.fields.iter().enumerate() {
                let offset = LLVMOffsetOfElement(data_layout, llvm_type, i as u32) as usize;

                let byte_offset = byte_offset + offset;
                // We get the remainder here since we consider the offset inside the word
                let num = classify_merge(
                    context,
                    source,
                    structs,
                    data_layout,
                    &field.ty,
                    &mut subpieces,
                    byte_offset % 8,
                );

                if num == 0 {
                    return 0;
                }

                let pos = byte_offset / 8;
                let mut i = 0;
                while i < num && (i + pos) < words {
                    let subpiece = &subpieces[i];
                    let piece = &mut pieces[i + pos];
                    piece.vec = (piece.vec || subpiece.vec)
                        && piece.class == X64Class::CLASS_SSE
                        && subpiece.class == X64Class::CLASS_SSE;
                    piece.class = merge_classes(subpiece.class, piece.class);
                    piece.meaningful_bits = piece.meaningful_bits.max(subpiece.meaningful_bits);
                    i += 1;
                }
            }

            // Final merger cleanup
            for i in 0..words {
                /* If one class is MEMORY, everything should be passed in
                memory.  */
                if pieces[i].class == X64Class::CLASS_MEMORY {
                    return 0;
                }
            }

            words
        },
    }
}

fn merge_classes(class1: X64Class, class2: X64Class) -> X64Class {
    // Rule #1: If both classes are equal, this is the resulting class.
    if class1 == class2 {
        return class1;
    }

    // Rule #2: If one of the classes is NO_CLASS, the resulting class is the other class.
    if class1 == X64Class::CLASS_NO_CLASS {
        return class2;
    }
    if class2 == X64Class::CLASS_NO_CLASS {
        return class1;
    }

    // Rule #3: If one of the classes is MEMORY, the result is MEMORY.
    if class1 == X64Class::CLASS_MEMORY || class2 == X64Class::CLASS_MEMORY {
        return X64Class::CLASS_MEMORY;
    }

    // Rule #4: If one of the classes is INTEGER, the result is INTEGER.
    if class1 == X64Class::CLASS_INTEGER || class2 == X64Class::CLASS_INTEGER {
        return X64Class::CLASS_INTEGER;
    }

    // Rule #5: If one of the classes is X87, X87UP, or COMPLEX_X87 class,
    // MEMORY is used. I don't use X87 anywhere.

    // Rule #6: Otherwise class SSE is used.
    X64Class::CLASS_SSE
}

pub fn add_fn_decl(
    context: LLVMContextRef,
    module: LLVMModuleRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
    decl: &TypedFn,
) -> (LLVMValueRef, FnABIInfo) {
    unsafe {
        let fn_abi_info = create_abi_info(
            &decl.args,
            decl.variadic,
            &decl.return_ty,
            context,
            source,
            structs,
            data_layout,
        );

        let name = CString::new(source[decl.ident.clone()].to_owned()).unwrap();
        let fn_val = LLVMAddFunction(module, name.as_ptr(), fn_abi_info.llvm_ty);

        for (attr, i) in &fn_abi_info.attributes {
            LLVMAddAttributeAtIndex(fn_val, *i, *attr);
        }
        (fn_val, fn_abi_info)
    }
}

pub fn add_fn_prologue(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    decl: &TypedFn,
    fn_ref: LLVMValueRef,
    abi_info: &FnABIInfo,
    vars: &mut Vec<Variable>,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
) {
    unsafe {
        let mut ir_param = 0;
        if let Piece {
            class: X64Class::CLASS_MEMORY,
            ..
        } = abi_info.ret.pieces[0]
        {
            ir_param = 1;
        }

        for (abi, arg) in abi_info.args.iter().zip(&decl.args) {
            if let Piece {
                class: X64Class::CLASS_MEMORY,
                ..
            } = abi.pieces[0]
            {
                vars.push(Variable {
                    ident: arg.ident.clone(),
                    ty: arg.ty.clone(),
                    ptr: LLVMGetParam(fn_ref, ir_param),
                });
                ir_param += 1;
                continue;
            }

            let ptr = entry_alloca(
                context,
                fn_ref,
                lower::to_llvm_type(context, &arg.ty, source, structs),
                CString::new(source[arg.ident.clone()].to_owned())
                    .unwrap()
                    .as_ptr(),
            );

            vars.push(Variable {
                ident: arg.ident.clone(),
                ty: arg.ty.clone(),
                ptr,
            });

            if arg.ty.is_scalar() {
                let param = LLVMGetParam(fn_ref, ir_param);
                LLVMBuildStore(builder, param, ptr);
                ir_param += 1;
                continue;
            }
            //Reconstruct the aggregate from ABI complaint IR args
            //There might be problems if the struct isn't aligned, but I think it always
            //is in my case
            if abi.single_piece {
                match abi.pieces[0].class {
                    X64Class::CLASS_NO_CLASS => {}
                    X64Class::CLASS_MEMORY => unreachable!(),
                    X64Class::CLASS_INTEGER => {
                        let meaningful_bits_type =
                            LLVMIntTypeInContext(context, abi.pieces[0].meaningful_bits as u32);

                        let param = LLVMBuildTrunc(
                            builder,
                            LLVMGetParam(fn_ref, ir_param),
                            meaningful_bits_type,
                            c"int_bits".as_ptr(),
                        );
                        LLVMBuildStore(builder, param, ptr);
                        ir_param += 1;
                    }
                    X64Class::CLASS_SSE => {
                        // Simply use the IR param type
                        let param = LLVMGetParam(fn_ref, ir_param);
                        LLVMBuildStore(builder, param, ptr);
                        ir_param += 1;
                    }
                }
            } else {
                // Index the alloca'd struct ptr as if it's an array of
                // eightbytes and store the values piece by piece
                let word_type = LLVMInt64TypeInContext(context);
                for word in 0..2 {
                    let mut index = LLVMConstInt(LLVMInt32TypeInContext(context), word as u64, 0);
                    let mut ptr =
                        LLVMBuildGEP2(builder, word_type, ptr, &mut index, 1, c"gep".as_ptr());

                    match abi.pieces[word].class {
                        X64Class::CLASS_NO_CLASS | X64Class::CLASS_MEMORY => {
                            unreachable!()
                        }
                        X64Class::CLASS_INTEGER => {
                            // IR param is always i64, but I might need to store less
                            // bits. TODO: Possibly just change the IR param type
                            // itllvmgen instead of truncating here.
                            let meaningful_bits_type = LLVMIntTypeInContext(
                                context,
                                abi.pieces[word].meaningful_bits as u32,
                            );

                            ptr = LLVMBuildBitCast(
                                builder,
                                ptr,
                                LLVMPointerType(meaningful_bits_type, 0),
                                c"int_ptr".as_ptr(),
                            );

                            let param = LLVMBuildTrunc(
                                builder,
                                LLVMGetParam(fn_ref, ir_param),
                                meaningful_bits_type,
                                c"int_bits".as_ptr(),
                            );
                            LLVMBuildStore(builder, param, ptr);
                            ir_param += 1;
                        }
                        X64Class::CLASS_SSE => {
                            // Simply use the IR param type
                            let param = LLVMGetParam(fn_ref, ir_param);
                            LLVMBuildStore(builder, param, ptr);
                            ir_param += 1;
                        }
                    }
                }
            }
        }

        // LLVM docs recommend to put all alloca at the function beginning
        for var in &decl.scope.vars[decl.args.len()..] {
            let ptr = entry_alloca(
                context,
                fn_ref,
                lower::to_llvm_type(context, var.ty.as_ref().unwrap(), source, structs),
                CString::new(source[var.ident.clone()].to_owned())
                    .unwrap()
                    .as_ptr(),
            );

            vars.push(Variable {
                ident: var.ident.clone(),
                ty: var.ty.clone().unwrap(),
                ptr,
            });
        }
    }
}

fn entry_alloca(
    context: LLVMContextRef,
    fn_ref: LLVMValueRef,
    ty: LLVMTypeRef,
    name: *const std::ffi::c_char,
) -> LLVMValueRef {
    unsafe {
        let entry_builder = LLVMCreateBuilderInContext(context);
        let entry_block = LLVMGetEntryBasicBlock(fn_ref);
        let instr = LLVMGetFirstInstruction(entry_block);
        if instr.is_null() {
            LLVMPositionBuilderAtEnd(entry_builder, entry_block);
        } else {
            LLVMPositionBuilderBefore(entry_builder, instr);
        }
        let val = LLVMBuildAlloca(entry_builder, ty, name);
        LLVMDisposeBuilder(entry_builder);
        val
    }
}

pub fn lower_ret(
    expr: &Option<TypedExpr>,
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    vars: &mut Vec<Variable>,
    fn_ref: LLVMValueRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    fn_abi_info: &FnABIInfo,
    data_layout: LLVMTargetDataRef,
) {
    unsafe {
        let Some(expr) = expr else {
            LLVMBuildRetVoid(builder);
            return;
        };
        let value = lower::lower_expr(
            expr,
            context,
            module,
            builder,
            vars,
            fn_ref,
            source,
            structs,
            data_layout,
        );
        if expr.ty.is_scalar() {
            LLVMBuildRet(builder, value);
            return;
        }
        if let Piece {
            class: X64Class::CLASS_MEMORY,
            ..
        } = fn_abi_info.ret.pieces[0]
        {
            LLVMBuildStore(builder, value, LLVMGetParam(fn_ref, 0));
            LLVMBuildRetVoid(builder);
            return;
        }
        let src_type = lower::to_llvm_type(context, &expr.ty, source, structs);

        let src = entry_alloca(context, fn_ref, src_type, c"retval".as_ptr());
        let dst_type = LLVMGetReturnType(LLVMGlobalGetValueType(fn_ref));
        let dst = entry_alloca(context, fn_ref, dst_type, c"retval.coerce".as_ptr());
        LLVMBuildStore(builder, value, src);
        // Clang casts it by doing memcpy so
        LLVMBuildMemCpy(
            builder,
            dst,
            LLVMGetAlignment(dst),
            src,
            LLVMGetAlignment(src),
            LLVMSizeOf(src_type),
        );
        LLVMBuildRet(
            builder,
            LLVMBuildLoad2(builder, dst_type, dst, c"coerce".as_ptr()),
        );
    }
}

pub fn call(
    fn_ptr: LLVMValueRef,
    fn_ty: &Type,
    args_exprs: &Vec<TypedExpr>,
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    vars: &mut Vec<Variable>,
    caller_fn_ref: LLVMValueRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
) -> LLVMValueRef {
    unsafe {
        let Type::Fn {
            args,
            variadic,
            return_ty,
        } = fn_ty
        else {
            unreachable!()
        };

        let fn_abi_info = create_abi_info(
            args,
            *variadic,
            return_ty,
            context,
            source,
            structs,
            data_layout,
        );

        let used_abi_info = if *variadic {
            let mut args = Vec::new();
            for arg in args_exprs {
                args.push(FieldDecl {
                    ident: 0..0,
                    ty: arg.ty.clone(),
                });
            }

            &create_abi_info(
                &args,
                false,
                return_ty,
                context,
                source,
                structs,
                data_layout,
            )
        } else {
            &fn_abi_info
        };
        // let func_c_str = CString::new(func.clone()).unwrap();
        // let called_fn_ref = LLVMGetNamedFunction(llvmgen.module, func_c_str.as_ptr());

        let param_types_count = LLVMCountParamTypes(used_abi_info.llvm_ty) as usize;
        let mut param_types = Vec::with_capacity(param_types_count);
        LLVMGetParamTypes(used_abi_info.llvm_ty, param_types.as_mut_ptr());
        param_types.set_len(param_types_count);

        let mut args_ptr: Vec<LLVMValueRef> = Vec::new();
        let ret_ptr = if return_ty.is_scalar() {
            None
        } else {
            if let Piece {
                class: X64Class::CLASS_MEMORY,
                ..
            } = used_abi_info.ret.pieces[0]
            {
                let dst = entry_alloca(
                    context,
                    caller_fn_ref,
                    lower::to_llvm_type(context, return_ty, source, structs),
                    c"ret".as_ptr(),
                );
                args_ptr.push(dst);
                Some(dst)
            } else {
                None
            }
        };

        for (abi, expr) in used_abi_info.args.iter().zip(args_exprs) {
            let val = lower::lower_expr(
                expr,
                context,
                module,
                builder,
                vars,
                caller_fn_ref,
                source,
                structs,
                data_layout,
            );
            if expr.ty.is_scalar() {
                args_ptr.push(val);
                continue;
            }

            if abi.single_piece {
                match abi.pieces[0].class {
                    X64Class::CLASS_NO_CLASS => {}
                    X64Class::CLASS_MEMORY => {
                        let alloca = entry_alloca(
                            context,
                            caller_fn_ref,
                            lower::to_llvm_type(context, &expr.ty, source, structs),
                            c"arg".as_ptr(),
                        );
                        LLVMBuildStore(builder, val, alloca);
                        args_ptr.push(alloca);
                    }
                    X64Class::CLASS_INTEGER => {
                        // Since the argument isn't a scalar, I probably always have to reinterpret
                        // it as such by using store & load
                        // let ext = LLVMBuildZExtOrBitCast(
                        //     builder,
                        //     val,
                        //     LLVMInt64TypeInContext(context),
                        //     c"extA".as_ptr(),
                        // );
                        // args_ptr.push(ext);

                        let alloca = entry_alloca(
                            context,
                            caller_fn_ref,
                            lower::to_llvm_type(context, &expr.ty, source, structs),
                            c"arg".as_ptr(),
                        );
                        LLVMBuildStore(builder, val, alloca);
                        let val = LLVMBuildLoad2(
                            builder,
                            LLVMInt64TypeInContext(context),
                            alloca,
                            c"arg_i64".as_ptr(),
                        );
                        args_ptr.push(val);
                    }
                    X64Class::CLASS_SSE => {
                        args_ptr.push(val);
                    }
                }
            } else {
                let alloca = entry_alloca(
                    context,
                    caller_fn_ref,
                    lower::to_llvm_type(context, &expr.ty, source, structs),
                    c"arg".as_ptr(),
                );
                LLVMBuildStore(builder, val, alloca);

                // Index the alloca'd struct ptr as if it's an array of
                // eightbytes and load the values piece by piece
                let word_type = LLVMInt64TypeInContext(context);
                for word in 0..2 {
                    let mut index = LLVMConstInt(LLVMInt32TypeInContext(context), word as u64, 0);
                    let ptr =
                        LLVMBuildGEP2(builder, word_type, alloca, &mut index, 1, c"gep".as_ptr());
                    match abi.pieces[word].class {
                        X64Class::CLASS_NO_CLASS | X64Class::CLASS_MEMORY => {
                            unreachable!()
                        }
                        X64Class::CLASS_INTEGER => {
                            // IR param is always i64, but I might need to store less
                            // bits. TODO: Possibly just change the IR param type
                            // itllvmgen instead of extending here.
                            let meaningful_bits_type = LLVMIntTypeInContext(
                                context,
                                abi.pieces[word].meaningful_bits as u32,
                            );
                            let val = LLVMBuildLoad2(
                                builder,
                                meaningful_bits_type,
                                ptr,
                                c"int_val".as_ptr(),
                            );
                            let ext = LLVMBuildZExtOrBitCast(
                                builder,
                                val,
                                LLVMInt64TypeInContext(context),
                                c"extB".as_ptr(),
                            );
                            args_ptr.push(ext);
                        }
                        X64Class::CLASS_SSE => {
                            // let p = LLVMGetParam(fn_ptr, args_ptr.len() as u32);
                            // let ty = LLVMTypeOf(p);
                            let val = LLVMBuildLoad2(
                                builder,
                                param_types[args_ptr.len()],
                                ptr,
                                c"float_val".as_ptr(),
                            );
                            args_ptr.push(val);
                        }
                    }
                }
            }
        }

        if return_ty.is_scalar() {
            LLVMBuildCall2(
                builder,
                fn_abi_info.llvm_ty,
                fn_ptr,
                args_ptr.as_mut_ptr(),
                args_ptr.len() as u32,
                c"".as_ptr(),
            )
        } else if let Some(dst) = ret_ptr {
            LLVMBuildCall2(
                builder,
                fn_abi_info.llvm_ty,
                fn_ptr,
                args_ptr.as_mut_ptr(),
                args_ptr.len() as u32,
                c"".as_ptr(),
            );
            LLVMBuildLoad2(
                builder,
                lower::to_llvm_type(context, return_ty, source, structs),
                dst,
                c"call_retval".as_ptr(),
            )
        } else {
            let call_val = LLVMBuildCall2(
                builder,
                fn_abi_info.llvm_ty,
                fn_ptr,
                args_ptr.as_mut_ptr(),
                args_ptr.len() as u32,
                c"".as_ptr(),
            );

            // Clang casts it by doing memcpy so
            let src_type = LLVMGetReturnType(fn_abi_info.llvm_ty);
            let src = entry_alloca(context, caller_fn_ref, src_type, c"callval.coerce".as_ptr());
            let dst_type = lower::to_llvm_type(context, return_ty, source, structs);
            let dst = entry_alloca(context, caller_fn_ref, dst_type, c"callval".as_ptr());

            LLVMBuildStore(builder, call_val, src);
            LLVMBuildMemCpy(
                builder,
                dst,
                LLVMGetAlignment(dst),
                src,
                LLVMGetAlignment(src),
                LLVMSizeOf(dst_type),
            );

            LLVMBuildLoad2(
                builder,
                lower::to_llvm_type(context, return_ty, source, structs),
                dst,
                c"call_retval".as_ptr(),
            )
        }
    }
}
