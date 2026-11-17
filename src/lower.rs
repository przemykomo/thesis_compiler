use std::collections::HashMap;
use std::ffi::CString;
use std::ptr;

use llvm_sys::LLVMIntPredicate;
use llvm_sys::LLVMRealPredicate;
use llvm_sys::analysis::LLVMVerifierFailureAction;
use llvm_sys::analysis::LLVMVerifyModule;
use llvm_sys::prelude::*;
use llvm_sys::target_machine::*;
use llvm_sys::{core::*, target::*};
use std::ops::Range;

use crate::abi::*;
use crate::ast::ArithmeticOp;
use crate::ast::BooleanOp;
use crate::ast::ComparisonOp;
use crate::ast::StructDecl;
use crate::ast::Type;
use crate::csa::*;

#[derive(Debug)]
pub struct Variable {
    pub ident: Range<usize>,
    pub ty: Type,
    pub ptr: LLVMValueRef,
}

pub fn lower_to_llvm(
    typed_ast: TypedAST,
    source: &str,
) -> (LLVMContextRef, LLVMTargetMachineRef, LLVMModuleRef) {
    unsafe {
        let context = LLVMContextCreate();
        let builder = LLVMCreateBuilderInContext(context);
        let module = LLVMModuleCreateWithNameInContext(c"main".as_ptr(), context);

        LLVMInitializeX86Target();
        LLVMInitializeX86TargetInfo();
        LLVMInitializeX86AsmPrinter();
        // LLVMInitializeX86AsmParser();
        // LLVMInitializeX86Disassembler();
        LLVMInitializeX86TargetMC();

        let mut target = ptr::null_mut();
        let mut err_string = ::core::ptr::null_mut();

        let triple = c"x86_64-pc-linux-gnu".as_ptr();
        let ret = LLVMGetTargetFromTriple(triple, &mut target, &mut err_string);
        if ret == 1 {
            panic!("get target");
        }

        let opt = LLVMCreateTargetMachineOptions();
        LLVMTargetMachineOptionsSetCPU(opt, c"x86-64".as_ptr());
        LLVMTargetMachineOptionsSetABI(opt, c"sysv".as_ptr());
        LLVMTargetMachineOptionsSetRelocMode(opt, LLVMRelocMode::LLVMRelocPIC);
        LLVMTargetMachineOptionsSetCodeGenOptLevel(opt, LLVMCodeGenOptLevel::LLVMCodeGenLevelNone);

        let machine = LLVMCreateTargetMachineWithOptions(target, triple, opt);

        let data_layout = LLVMCreateTargetDataLayout(machine);
        LLVMSetDataLayout(module, LLVMCopyStringRepOfTargetData(data_layout));

        LLVMSetTarget(module, triple);
        let mut label = 0;

        let mut declarations = Vec::new();

        for typed_fn in typed_ast.functions.values() {
            let (fn_ref, abi_info) = add_fn_decl(
                context,
                module,
                source,
                &typed_ast.structs,
                data_layout,
                typed_fn,
            );

            declarations.push((fn_ref, abi_info, typed_fn.ident.clone()));
        }

        for (fn_ref, abi_info, ident) in declarations {
            let typed_fn = typed_ast.functions.get(&source[ident]).unwrap();
            if let Some(body) = &typed_fn.body {
                let mut vars: Vec<Variable> = Vec::new();

                let entry_block = LLVMAppendBasicBlockInContext(context, fn_ref, c"entry".as_ptr());
                LLVMPositionBuilderAtEnd(builder, entry_block);

                add_fn_prologue(
                    context,
                    builder,
                    typed_fn,
                    fn_ref,
                    &abi_info,
                    &mut vars,
                    source,
                    &typed_ast.structs,
                );

                for statement in &body.statements {
                    lower_statement(
                        statement,
                        context,
                        module,
                        builder,
                        &mut vars,
                        fn_ref,
                        &abi_info,
                        source,
                        &mut label,
                        None,
                        &typed_ast.structs,
                        data_layout,
                    );
                }
                // if LLVMVerifyFunction(fn_ref, LLVMVerifierFailureAction::LLVMPrintMessageAction)
                //     == 1
                // {
                //     LLVMDumpModule(module);
                //     LLVMDeleteFunction(fn_ref);
                //     panic!("Couldn't verify a function");
                // }
            }
        }

        if LLVMVerifyModule(
            module,
            LLVMVerifierFailureAction::LLVMPrintMessageAction,
            std::ptr::null_mut(),
        ) == 1
        {
            LLVMDumpModule(module);
            panic!("Coudln't verify the module");
        }
        (context, machine, module)
    }
}

#[derive(Clone, Copy)]
struct LoopCtx {
    cond_block: LLVMBasicBlockRef,
    end_block: LLVMBasicBlockRef,
}

fn lower_statement(
    statement: &TypedStatement,
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    vars: &mut Vec<Variable>,
    fn_ref: LLVMValueRef,
    abi_info: &FnABIInfo,
    source: &str,
    label: &mut usize,
    loop_ctx: Option<LoopCtx>,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
) {
    unsafe {
        match &statement {
            TypedStatement::Let { ident, expr } => {
                if let Some(expr) = expr {
                    let ptr = vars
                        .iter()
                        .find(|v| &source[v.ident.clone()] == &source[ident.clone()])
                        .unwrap()
                        .ptr;
                    let value = lower_expr(
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
                    LLVMBuildStore(builder, value, ptr);
                }
            }
            TypedStatement::If {
                expr,
                body,
                else_body,
            } => {
                let condition = lower_expr(
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
                let then_block = LLVMCreateBasicBlockInContext(
                    context,
                    CString::new(format!("if.then{}", *label)).unwrap().as_ptr(),
                );

                let else_block = LLVMCreateBasicBlockInContext(
                    context,
                    CString::new(format!("if.else{}", *label)).unwrap().as_ptr(),
                );

                let end_block = LLVMCreateBasicBlockInContext(
                    context,
                    CString::new(format!("if.end{}", *label)).unwrap().as_ptr(),
                );

                *label += 1;

                let br_block = if else_body.is_some() {
                    else_block
                } else {
                    end_block
                };

                LLVMBuildCondBr(builder, condition, then_block, br_block);

                LLVMAppendExistingBasicBlock(fn_ref, then_block);
                LLVMPositionBuilderAtEnd(builder, then_block);
                for statement in &body.statements {
                    lower_statement(
                        statement,
                        context,
                        module,
                        builder,
                        vars,
                        fn_ref,
                        abi_info,
                        source,
                        label,
                        loop_ctx,
                        structs,
                        data_layout,
                    );
                }

                if body.return_actuality != BlockReturnActuality::AlwaysReturns {
                    LLVMBuildBr(builder, end_block);
                }

                let mut else_always_returns = false;

                if let Some(else_body) = else_body {
                    LLVMAppendExistingBasicBlock(fn_ref, else_block);
                    LLVMPositionBuilderAtEnd(builder, else_block);
                    for statement in &else_body.statements {
                        lower_statement(
                            statement,
                            context,
                            module,
                            builder,
                            vars,
                            fn_ref,
                            abi_info,
                            source,
                            label,
                            loop_ctx,
                            structs,
                            data_layout,
                        );
                    }
                    if else_body.return_actuality == BlockReturnActuality::AlwaysReturns {
                        else_always_returns = true;
                    } else {
                        LLVMBuildBr(builder, end_block);
                    }
                }

                if body.return_actuality != BlockReturnActuality::AlwaysReturns
                    || !else_always_returns
                {
                    LLVMAppendExistingBasicBlock(fn_ref, end_block);
                    LLVMPositionBuilderAtEnd(builder, end_block);
                }
            }
            TypedStatement::While { expr, body } => {
                let cond_block = LLVMCreateBasicBlockInContext(
                    context,
                    CString::new(format!("while.cond{}", *label))
                        .unwrap()
                        .as_ptr(),
                );

                let body_block = LLVMCreateBasicBlockInContext(
                    context,
                    CString::new(format!("while.body{}", *label))
                        .unwrap()
                        .as_ptr(),
                );

                let end_block = LLVMCreateBasicBlockInContext(
                    context,
                    CString::new(format!("while.end{}", *label))
                        .unwrap()
                        .as_ptr(),
                );

                *label += 1;

                LLVMAppendExistingBasicBlock(fn_ref, cond_block);
                LLVMPositionBuilderAtEnd(builder, cond_block);
                let condition = lower_expr(
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
                LLVMBuildCondBr(builder, condition, body_block, end_block);

                LLVMAppendExistingBasicBlock(fn_ref, body_block);
                LLVMPositionBuilderAtEnd(builder, body_block);

                for statement in &body.statements {
                    lower_statement(
                        statement,
                        context,
                        module,
                        builder,
                        vars,
                        fn_ref,
                        abi_info,
                        source,
                        label,
                        Some(LoopCtx {
                            cond_block,
                            end_block,
                        }),
                        structs,
                        data_layout,
                    );
                }

                LLVMBuildBr(builder, cond_block);

                LLVMAppendExistingBasicBlock(fn_ref, end_block);
                LLVMPositionBuilderAtEnd(builder, end_block);
            }
            TypedStatement::Return(expr) => {
                lower_ret(
                    expr,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    abi_info,
                    data_layout,
                );
            }
            TypedStatement::Break => {
                LLVMBuildBr(builder, loop_ctx.unwrap().end_block);
            }
            TypedStatement::Continue => {
                LLVMBuildBr(builder, loop_ctx.unwrap().cond_block);
            }
            TypedStatement::Expr(expr) => {
                lower_expr(
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
            }
        }
    }
}

pub fn lower_expr(
    expr: &TypedExpr,
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    vars: &mut Vec<Variable>,
    fn_ref: LLVMValueRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
) -> LLVMValueRef {
    unsafe {
        match &expr.kind {
            TypedExprKind::IntLiteral(val) => LLVMConstInt(
                to_llvm_type(context, &expr.ty, source, structs),
                *val as u64,
                1,
            ),
            TypedExprKind::CharLiteral(val) => {
                LLVMConstInt(LLVMInt8TypeInContext(context), *val as u64, 0)
            }
            TypedExprKind::BoolLiteral(val) => {
                LLVMConstInt(LLVMInt1TypeInContext(context), *val as u64, 0)
            }
            TypedExprKind::FloatLiteral(val) => {
                LLVMConstReal(to_llvm_type(context, &expr.ty, source, structs), *val)
            }
            TypedExprKind::StringLiteral(s) => LLVMBuildGlobalString(
                builder,
                CString::new(s.clone()).unwrap().as_ptr(), //TODO remove clone
                c"str".as_ptr(),
            ),
            TypedExprKind::Call { lhs, args } => {
                let fn_ptr = lower_expr(
                    lhs,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );

                call(
                    fn_ptr,
                    &lhs.ty,
                    args,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                )
            }
            TypedExprKind::Arithmetic { lhs, rhs, op } => {
                let lhs = lower_expr(
                    lhs,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );
                let rhs = lower_expr(
                    rhs,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );

                lower_arithmetic_expr(&expr.ty, builder, op, lhs, rhs)
            }
            TypedExprKind::Comparison { lhs, rhs, op } => {
                let lhs_v = lower_expr(
                    lhs,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );
                let rhs_v = lower_expr(
                    rhs,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );

                if lhs.ty.is_int() {
                    use LLVMIntPredicate::*;
                    let op = if lhs.ty.is_signed() {
                        match op {
                            ComparisonOp::Eq => LLVMIntEQ,
                            ComparisonOp::Greater => LLVMIntSGT,
                            ComparisonOp::Less => LLVMIntSLT,
                            ComparisonOp::GreaterEq => LLVMIntSGE,
                            ComparisonOp::LessEq => LLVMIntSLE,
                            ComparisonOp::NotEq => LLVMIntNE,
                        }
                    } else {
                        match op {
                            ComparisonOp::Eq => LLVMIntEQ,
                            ComparisonOp::Greater => LLVMIntUGT,
                            ComparisonOp::Less => LLVMIntULT,
                            ComparisonOp::GreaterEq => LLVMIntUGE,
                            ComparisonOp::LessEq => LLVMIntULE,
                            ComparisonOp::NotEq => LLVMIntNE,
                        }
                    };

                    LLVMBuildICmp(builder, op, lhs_v, rhs_v, c"cmp".as_ptr())
                } else if lhs.ty.is_float() {
                    use LLVMRealPredicate::*;
                    let op = match op {
                        ComparisonOp::Eq => LLVMRealOEQ,
                        ComparisonOp::Greater => LLVMRealOGT,
                        ComparisonOp::Less => LLVMRealOLT,
                        ComparisonOp::GreaterEq => LLVMRealOGT,
                        ComparisonOp::LessEq => LLVMRealOLE,
                        ComparisonOp::NotEq => LLVMRealONE,
                    };
                    LLVMBuildFCmp(builder, op, lhs_v, rhs_v, c"cmp".as_ptr())
                } else {
                    unreachable!();
                }
            }
            TypedExprKind::Boolean { lhs, rhs, op } => {
                let lhs = lower_expr(
                    lhs,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );
                let rhs = lower_expr(
                    rhs,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );

                match op {
                    BooleanOp::And => LLVMBuildAnd(builder, lhs, rhs, c"and".as_ptr()),
                    BooleanOp::Or => LLVMBuildOr(builder, lhs, rhs, c"or".as_ptr()),
                }
            }
            TypedExprKind::Assign { lhs, rhs } => {
                let ptr = lower_place(
                    lhs,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );
                let value = lower_expr(
                    rhs,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );

                LLVMBuildStore(builder, value, ptr);
                value
            }
            TypedExprKind::ArithmeticAssign { lhs, rhs, op } => {
                let ty = &lhs.ty;
                let ptr = lower_place(
                    lhs,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );
                let lhs = LLVMBuildLoad2(
                    builder,
                    to_llvm_type(context, &lhs.ty, source, structs),
                    ptr,
                    c"lhs".as_ptr(),
                );
                let rhs = lower_expr(
                    rhs,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );

                let value = lower_arithmetic_expr(ty, builder, op, lhs, rhs);

                LLVMBuildStore(builder, value, ptr);
                value
            }
            TypedExprKind::Minus(expr) => {
                let value = lower_expr(
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
                LLVMBuildNeg(builder, value, c"neg".as_ptr())
            }
            TypedExprKind::Not(expr) => {
                let value = lower_expr(
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
                LLVMBuildNot(builder, value, c"not".as_ptr())
            }
            TypedExprKind::AddressOf(place) => lower_place(
                place,
                context,
                module,
                builder,
                vars,
                fn_ref,
                source,
                structs,
                data_layout,
            ),
            TypedExprKind::Deref(expr) => {
                let ty = to_llvm_type(context, &expr.ty, source, structs);
                let ptr = lower_expr(
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
                LLVMBuildLoad2(builder, ty, ptr, c"deref".as_ptr())
            }
            TypedExprKind::Cast { expr, ty } => {
                let src_ty = &expr.ty;
                let value = lower_expr(
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

                if let (Type::Pointer(_), Type::Pointer(_)) = (src_ty, ty) {
                    LLVMBuildBitCast(
                        builder,
                        value,
                        to_llvm_type(context, ty, source, structs),
                        c"bitcast".as_ptr(),
                    )
                } else if src_ty.is_int_char_bool() && ty.is_int_char_bool() {
                    LLVMBuildIntCast2(
                        builder,
                        value,
                        to_llvm_type(context, ty, source, structs),
                        src_ty.is_signed() as i32,
                        c"intcast".as_ptr(),
                    )
                } else if src_ty.is_float() && ty.is_float() {
                    LLVMBuildFPCast(
                        builder,
                        value,
                        to_llvm_type(context, ty, source, structs),
                        c"fpcast".as_ptr(),
                    )
                } else if src_ty.is_int() && ty.is_float() {
                    if src_ty.is_signed() {
                        LLVMBuildSIToFP(
                            builder,
                            value,
                            to_llvm_type(context, ty, source, structs),
                            c"sitofp".as_ptr(),
                        )
                    } else {
                        LLVMBuildUIToFP(
                            builder,
                            value,
                            to_llvm_type(context, ty, source, structs),
                            c"uitofp".as_ptr(),
                        )
                    }
                } else if src_ty.is_float() && ty.is_int() {
                    if ty.is_signed() {
                        LLVMBuildFPToSI(
                            builder,
                            value,
                            to_llvm_type(context, ty, source, structs),
                            c"fptosi".as_ptr(),
                        )
                    } else {
                        LLVMBuildFPToUI(
                            builder,
                            value,
                            to_llvm_type(context, ty, source, structs),
                            c"fptoui".as_ptr(),
                        )
                    }
                } else {
                    unreachable!()
                }
            }
            TypedExprKind::StructLiteral { fields } => {
                let mut val = LLVMGetPoison(to_llvm_type(context, &expr.ty, source, structs));

                for (i, expr) in fields.iter().enumerate() {
                    val = LLVMBuildInsertValue(
                        builder,
                        val,
                        lower_expr(
                            expr,
                            context,
                            module,
                            builder,
                            vars,
                            fn_ref,
                            source,
                            structs,
                            data_layout,
                        ),
                        i as u32,
                        c"".as_ptr(),
                    );
                }

                val
            }
            TypedExprKind::Variable(v) => {
                let var = &vars[*v];
                LLVMBuildLoad2(
                    builder,
                    to_llvm_type(context, &var.ty, source, structs),
                    var.ptr,
                    CString::new(source[var.ident.clone()].to_owned())
                        .unwrap()
                        .as_ptr(),
                )
            }
            TypedExprKind::Function(range) => LLVMGetNamedFunction(
                module,
                CString::new(source[range.clone()].to_owned())
                    .unwrap()
                    .as_ptr(),
            ),
            TypedExprKind::Field { place, field } => {
                let ptr = lower_field_access(
                    place,
                    context,
                    module,
                    builder,
                    field.clone(),
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                );

                LLVMBuildLoad2(
                    builder,
                    to_llvm_type(context, &expr.ty, source, structs),
                    ptr,
                    CString::new(source[field.clone()].to_owned())
                        .unwrap()
                        .as_ptr(),
                )
            }
            TypedExprKind::IndexPtr { ptr, index } => {
                let llvm_ty = to_llvm_type(context, &expr.ty, source, structs);
                let ptr = lower_ptr_index(
                    llvm_ty,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                    ptr,
                    index,
                );
                LLVMBuildLoad2(builder, llvm_ty, ptr, c"".as_ptr())
            }
            TypedExprKind::IndexArr { place, index } => {
                let llvm_ty = to_llvm_type(context, &expr.ty, source, structs);
                let ptr = lower_array_index(
                    llvm_ty,
                    context,
                    module,
                    builder,
                    vars,
                    fn_ref,
                    source,
                    structs,
                    data_layout,
                    place,
                    index,
                );
                LLVMBuildLoad2(builder, llvm_ty, ptr, c"".as_ptr())
            }
        }
    }
}

fn lower_arithmetic_expr(
    ty: &Type,
    builder: LLVMBuilderRef,
    op: &ArithmeticOp,
    lhs: LLVMValueRef,
    rhs: LLVMValueRef,
) -> LLVMValueRef {
    use Type::*;
    unsafe {
        match ty {
            I8 | U8 | I16 | U16 | I32 | U32 | I64 | U64 => match op {
                ArithmeticOp::Add => LLVMBuildAdd(builder, lhs, rhs, c"add".as_ptr()),
                ArithmeticOp::Sub => LLVMBuildSub(builder, lhs, rhs, c"sub".as_ptr()),
                ArithmeticOp::Mul => LLVMBuildMul(builder, lhs, rhs, c"mul".as_ptr()),
                ArithmeticOp::Div => {
                    if ty.is_signed() {
                        LLVMBuildSDiv(builder, lhs, rhs, c"div".as_ptr())
                    } else {
                        LLVMBuildUDiv(builder, lhs, rhs, c"div".as_ptr())
                    }
                }
                ArithmeticOp::Rem => {
                    if ty.is_signed() {
                        LLVMBuildSRem(builder, lhs, rhs, c"rem".as_ptr())
                    } else {
                        LLVMBuildURem(builder, lhs, rhs, c"rem".as_ptr())
                    }
                }
            },
            F32 | F64 => match op {
                ArithmeticOp::Add => LLVMBuildFAdd(builder, lhs, rhs, c"add".as_ptr()),
                ArithmeticOp::Sub => LLVMBuildFSub(builder, lhs, rhs, c"sub".as_ptr()),
                ArithmeticOp::Mul => LLVMBuildFMul(builder, lhs, rhs, c"mul".as_ptr()),
                ArithmeticOp::Div => LLVMBuildFDiv(builder, lhs, rhs, c"div".as_ptr()),
                ArithmeticOp::Rem => LLVMBuildFRem(builder, lhs, rhs, c"rem".as_ptr()),
            },
            _ => unreachable!(),
        }
    }
}

fn lower_place(
    place: &Place,
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    vars: &mut Vec<Variable>,
    fn_ref: LLVMValueRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
) -> LLVMValueRef {
    match &place.kind {
        PlaceKind::Deref(expr) => lower_expr(
            expr,
            context,
            module,
            builder,
            vars,
            fn_ref,
            source,
            structs,
            data_layout,
        ),
        PlaceKind::Variable(v) => vars[*v].ptr,
        PlaceKind::IndexPtr(lhs, idx) => lower_ptr_index(
            to_llvm_type(context, &place.ty, source, structs),
            context,
            module,
            builder,
            vars,
            fn_ref,
            source,
            structs,
            data_layout,
            lhs,
            idx,
        ),
        PlaceKind::IndexArr(arr, idx) => lower_array_index(
            to_llvm_type(context, &place.ty, source, structs),
            context,
            module,
            builder,
            vars,
            fn_ref,
            source,
            structs,
            data_layout,
            arr,
            idx,
        ),
        PlaceKind::Field(place, range) => lower_field_access(
            place,
            context,
            module,
            builder,
            range.clone(),
            vars,
            fn_ref,
            source,
            structs,
            data_layout,
        ),
    }
}

fn lower_ptr_index(
    llvm_ty: LLVMTypeRef,
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    vars: &mut Vec<Variable>,
    fn_ref: LLVMValueRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
    lhs: &TypedExpr,
    idx: &TypedExpr,
) -> *mut llvm_sys::LLVMValue {
    let ptr = lower_expr(
        lhs,
        context,
        module,
        builder,
        vars,
        fn_ref,
        source,
        structs,
        data_layout,
    );
    let idx = lower_expr(
        idx,
        context,
        module,
        builder,
        vars,
        fn_ref,
        source,
        structs,
        data_layout,
    );
    let mut indices = [idx];

    unsafe { LLVMBuildGEP2(builder, llvm_ty, ptr, indices.as_mut_ptr(), 1, c"".as_ptr()) }
}

fn lower_array_index(
    llvm_ty: LLVMTypeRef,
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    vars: &mut Vec<Variable>,
    fn_ref: LLVMValueRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
    place: &Place,
    index: &TypedExpr,
) -> *mut llvm_sys::LLVMValue {
    let ptr = lower_place(
        place,
        context,
        module,
        builder,
        vars,
        fn_ref,
        source,
        structs,
        data_layout,
    );
    let idx = lower_expr(
        index,
        context,
        module,
        builder,
        vars,
        fn_ref,
        source,
        structs,
        data_layout,
    );
    let mut indices = [idx];

    unsafe { LLVMBuildGEP2(builder, llvm_ty, ptr, indices.as_mut_ptr(), 1, c"".as_ptr()) }
}

fn lower_field_access(
    place: &Place,
    context: LLVMContextRef,
    module: LLVMModuleRef,
    builder: LLVMBuilderRef,
    field: Range<usize>,
    vars: &mut Vec<Variable>,
    fn_ref: LLVMValueRef,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
    data_layout: LLVMTargetDataRef,
) -> LLVMValueRef {
    unsafe {
        let Type::Struct(ident) = &place.ty else {
            unreachable!()
        };

        let decl = structs.get(&source[ident.clone()]).unwrap();

        let ptr = lower_place(
            place,
            context,
            module,
            builder,
            vars,
            fn_ref,
            source,
            structs,
            data_layout,
        );
        let idx = decl
            .fields
            .iter()
            .position(|f| &source[f.ident.clone()] == &source[field.clone()])
            .unwrap();

        LLVMBuildStructGEP2(
            builder,
            to_llvm_type(context, &place.ty, source, structs),
            ptr,
            idx as u32,
            c"gep".as_ptr(),
        )
    }
}

pub fn to_llvm_type(
    context: LLVMContextRef,
    ty: &Type,
    source: &str,
    structs: &HashMap<&str, StructDecl>,
) -> LLVMTypeRef {
    unsafe {
        use Type::*;
        match ty {
            UnsizedInt | UnsizedFloat => unreachable!(),
            I8 | U8 | Char => LLVMInt8TypeInContext(context),
            I16 | U16 => LLVMInt16TypeInContext(context),
            I32 | U32 => LLVMInt32TypeInContext(context),
            I64 | U64 => LLVMInt64TypeInContext(context),
            F32 => LLVMFloatTypeInContext(context),
            F64 => LLVMDoubleTypeInContext(context),
            Pointer(_)
            | Fn {
                args: _,
                variadic: _,
                return_ty: _,
            } => LLVMPointerTypeInContext(context, 0),
            Boolean => LLVMInt1TypeInContext(context),
            Void => LLVMVoidTypeInContext(context),
            Struct(ident) => {
                let decl = structs.get(&source[ident.clone()]).unwrap();

                let mut fields: Vec<LLVMTypeRef> = decl
                    .fields
                    .iter()
                    .map(|f| to_llvm_type(context, &f.ty, source, structs))
                    .collect();

                LLVMStructTypeInContext(context, fields.as_mut_ptr(), fields.len() as u32, 0)
            }
            Array(ty, len) => {
                LLVMArrayType2(to_llvm_type(context, ty, source, structs), *len as u64)
            }
        }
    }
}
