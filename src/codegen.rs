use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::passes::PassManager;
use inkwell::targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, FloatValue, FunctionValue, PointerValue,
};
use inkwell::{AddressSpace, OptimizationLevel};

use crate::parser::*;

#[repr(C)]
pub struct UnoString {
    buffer: *mut u8,
    length: i32,
    max_length: i32,
}

impl UnoString {
    #[no_mangle]
    fn allocate_string(bytes: *const u8, length: i32) -> *mut UnoString {
        let ptr: *mut u8 = unsafe { libc::malloc(length as usize).cast() };

        unsafe {
            core::ptr::copy(bytes, ptr, length as usize);
        }

        let uno_string = UnoString {
            buffer: ptr,
            length,
            max_length: length,
        };

        let size = core::mem::size_of::<UnoString>();
        let ptr: *mut UnoString = unsafe { libc::malloc(size).cast() };

        unsafe { ptr.write(uno_string) };

        ptr
    }
}

#[no_mangle]
pub fn print(uno_string: *const UnoString) {
    let string = unsafe { &*uno_string };
    let slice = unsafe { std::slice::from_raw_parts(string.buffer, string.length as usize) };
    let str_value = std::str::from_utf8(slice);

    match str_value {
        Ok(value) => {
            println!("{}", value);
        }
        Err(_) => {}
    }
}

#[used]
static EXTERNAL_FNS1: [fn(*const UnoString); 1] = [print];

#[used]
static EXTERNAL_FNS2: [fn(bytes: *const u8, initial_length: i32) -> *mut UnoString; 1] =
    [UnoString::allocate_string];

pub enum ReturnValue<'a> {
    FloatValue(FloatValue<'a>),
    ArrayPtrValue(PointerValue<'a>),
    VoidValue,
}

pub struct Compiler<'a, 'ctx> {
    pub parser_result: &'a ParserResult,
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub fpm: &'a PassManager<FunctionValue<'ctx>>,
    pub llvm_module: &'a Module<'ctx>,
    pub fn_value_opt: Option<FunctionValue<'ctx>>,
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub fn compile(parser_result: ParserResult) {
        Target::initialize_all(&InitializationConfig::default());

        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple).unwrap();
        let target_machine = target
            .create_target_machine(
                &target_triple,
                TargetMachine::get_host_cpu_name().to_string().as_str(),
                TargetMachine::get_host_cpu_features().to_string().as_str(),
                OptimizationLevel::Default,
                RelocMode::Default,
                CodeModel::Default,
            )
            .unwrap();

        let context = Context::create();
        let module = context.create_module("uno");
        let builder = context.create_builder();
        let data_layout = &target_machine.get_target_data().get_data_layout();

        module.set_data_layout(data_layout);
        module.set_triple(&target_triple);

        let fpm = PassManager::create(&module);
        fpm.initialize();

        let string_struct_type = context.struct_type(
            &[
                context.i8_type().ptr_type(AddressSpace::default()).into(),
                context.i32_type().into(),
                context.i32_type().into(),
            ],
            false,
        );

        let struct_ptr_type = string_struct_type.ptr_type(AddressSpace::default());
        let bytes_ptr_type = context.i8_type().ptr_type(AddressSpace::default());
        let length_type = context.i32_type();

        let arg_ret_types = &[bytes_ptr_type.into(), length_type.into()];
        let alloc_string_fn_type = struct_ptr_type.fn_type(arg_ret_types, false);

        module.add_function("allocate_string", alloc_string_fn_type, None);

        let arg_ret_types = [struct_ptr_type.into()];
        let print_fn_type = context.void_type().fn_type(&arg_ret_types, false);

        module.add_function("print", print_fn_type, None);

        let mut compiler = Compiler {
            parser_result: &parser_result,
            context: &context,
            builder: &builder,
            fpm: &fpm,
            llvm_module: &module,
            fn_value_opt: None,
        };

        let void_return_type = context.void_type().fn_type(&[], false);
        let main_fn = module.add_function("main", void_return_type, None);

        let entry = context.append_basic_block(main_fn, "entry");

        builder.position_at_end(entry);

        for node in &compiler.parser_result.ast {
            compiler.compile_expr(node);
        }

        builder.build_return(None);

        println!("");
        println!("{}", module.print_to_string().to_string());
        println!("######");
        println!("");

        let ee = module
            .create_jit_execution_engine(OptimizationLevel::None)
            .unwrap();

        let maybe_fn = unsafe { ee.get_function::<unsafe extern "C" fn() -> f64>("main") };
        let compiled_fn = match maybe_fn {
            Ok(f) => f,
            Err(err) => {
                println!("Error: {:?}", err);
                std::process::exit(1);
            }
        };

        unsafe {
            compiled_fn.call();
        }
    }

    fn compile_expr(&mut self, expr: &Node) -> Result<ReturnValue<'ctx>, &'static str> {
        match *&expr {
            Node::InterpolableString(string) => {
                let i8_type = self.context.i8_type();
                let i8_array_type = i8_type.array_type(20);

                let hello_world_str = self.context.const_string(string.value.as_bytes(), false);
                let global_str = self.llvm_module.add_global(i8_array_type, None, "0");

                global_str.set_initializer(&hello_world_str);

                let malloc_string_fn = self.llvm_module.get_function("allocate_string").unwrap();
                let i32_type = self.context.i32_type();

                let zero = self.context.i32_type().const_int(0, false);
                let indices = [zero, zero];

                let element_pointer = unsafe {
                    self.builder
                        .build_gep(global_str.as_pointer_value(), &indices, "element_ptr")
                };

                let args = &[
                    element_pointer.into(),
                    i32_type
                        .const_int(string.value.len() as u64, false)
                        .as_basic_value_enum()
                        .into(),
                ];

                let uno_str_ptr = self
                    .builder
                    .build_call(malloc_string_fn, args, "tmp")
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                Ok(ReturnValue::ArrayPtrValue(
                    uno_str_ptr.as_basic_value_enum().into_pointer_value(),
                ))
            }

            Node::Call(call) => match self.get_function(call.fn_name.as_str()) {
                Some(fun) => {
                    let mut compiled_args = Vec::with_capacity(call.args.len());

                    for arg in &call.args {
                        compiled_args.push(self.compile_expr(&arg)?);
                    }

                    let argsv: Vec<BasicMetadataValueEnum> = compiled_args
                        .iter()
                        .map(|val| match *val {
                            ReturnValue::FloatValue(float_value) => float_value.into(),
                            ReturnValue::ArrayPtrValue(string_ptr) => string_ptr.into(),
                            _ => todo!(),
                        })
                        .collect();

                    match self
                        .builder
                        .build_call(fun, argsv.as_slice(), "tmp")
                        .try_as_basic_value()
                        .left()
                    {
                        Some(value) => match value {
                            inkwell::values::BasicValueEnum::PointerValue(value) => {
                                Ok(ReturnValue::ArrayPtrValue(value))
                            }
                            inkwell::values::BasicValueEnum::FloatValue(value) => {
                                Ok(ReturnValue::FloatValue(value))
                            }
                            _ => todo!(),
                        },
                        None => Ok(ReturnValue::VoidValue),
                    }
                }
                None => Err("Unknown function."),
            },
        }
    }

    #[inline]
    fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.llvm_module.get_function(name)
    }
}
