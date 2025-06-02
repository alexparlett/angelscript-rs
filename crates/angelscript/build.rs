use std::env;
use std::path::PathBuf;

fn main() {
    // Build AngelScript first
    let angelscript_dir = "vendor/angelscript/sdk/angelscript/source";

    // Determine the target architecture
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    // Build AngelScript library
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .include("vendor/angelscript/sdk/angelscript/include")
        .include("vendor/angelscript/sdk/add_on/scriptstdstring")
        .include("vendor/angelscript/sdk/add_on/scriptarray")
        .include("vendor/angelscript/sdk/add_on/scriptdictionary")
        .file(format!("{}/as_atomic.cpp", angelscript_dir))
        .file(format!("{}/as_builder.cpp", angelscript_dir))
        .file(format!("{}/as_bytecode.cpp", angelscript_dir))
        .file(format!("{}/as_callfunc.cpp", angelscript_dir))
        .file(format!("{}/as_callfunc_arm.cpp", angelscript_dir))
        .file(format!("{}/as_callfunc_mips.cpp", angelscript_dir))
        .file(format!("{}/as_callfunc_ppc.cpp", angelscript_dir))
        .file(format!("{}/as_callfunc_ppc_64.cpp", angelscript_dir))
        .file(format!("{}/as_callfunc_sh4.cpp", angelscript_dir))
        .file(format!("{}/as_callfunc_x86.cpp", angelscript_dir))
        .file(format!("{}/as_callfunc_x64_gcc.cpp", angelscript_dir))
        .file(format!("{}/as_callfunc_x64_mingw.cpp", angelscript_dir))
        .file(format!("{}/as_callfunc_x64_msvc.cpp", angelscript_dir))
        .file(format!("{}/as_compiler.cpp", angelscript_dir))
        .file(format!("{}/as_configgroup.cpp", angelscript_dir))
        .file(format!("{}/as_context.cpp", angelscript_dir))
        .file(format!("{}/as_datatype.cpp", angelscript_dir))
        .file(format!("{}/as_gc.cpp", angelscript_dir))
        .file(format!("{}/as_generic.cpp", angelscript_dir))
        .file(format!("{}/as_globalproperty.cpp", angelscript_dir))
        .file(format!("{}/as_memory.cpp", angelscript_dir))
        .file(format!("{}/as_module.cpp", angelscript_dir))
        .file(format!("{}/as_objecttype.cpp", angelscript_dir))
        .file(format!("{}/as_outputbuffer.cpp", angelscript_dir))
        .file(format!("{}/as_parser.cpp", angelscript_dir))
        .file(format!("{}/as_restore.cpp", angelscript_dir))
        .file(format!("{}/as_scriptcode.cpp", angelscript_dir))
        .file(format!("{}/as_scriptengine.cpp", angelscript_dir))
        .file(format!("{}/as_scriptfunction.cpp", angelscript_dir))
        .file(format!("{}/as_scriptnode.cpp", angelscript_dir))
        .file(format!("{}/as_scriptobject.cpp", angelscript_dir))
        .file(format!("{}/as_string.cpp", angelscript_dir))
        .file(format!("{}/as_string_util.cpp", angelscript_dir))
        .file(format!("{}/as_thread.cpp", angelscript_dir))
        .file(format!("{}/as_tokenizer.cpp", angelscript_dir))
        .file(format!("{}/as_typeinfo.cpp", angelscript_dir))
        .file(format!("{}/as_variablescope.cpp", angelscript_dir))
        .file("vendor/angelscript/sdk/add_on/scriptstdstring/scriptstdstring.cpp")
        .file("vendor/angelscript/sdk/add_on/scriptstdstring/scriptstdstring_utils.cpp")
        .file("vendor/angelscript/sdk/add_on/scriptarray/scriptarray.cpp")
        .file("vendor/angelscript/sdk/add_on/scriptdictionary/scriptdictionary.cpp")
        .flag_if_supported("-std=c++11");

    // Add architecture-specific files and flags
    if target_arch == "aarch64" && target_os == "macos" {
        // Use portable mode for ARM64 macOS to avoid assembly issues
        build.define("AS_MAX_PORTABILITY", Some("1"));
    } else if target_arch == "x86_64" {
        if target_os == "macos" {
            build.define("AS_APPLE", None);
        }
    }

    // macOS specific flags
    if target_os == "macos" {
        build.flag("-mmacosx-version-min=10.15");
        // Suppress warnings
        build.flag("-Wno-unused-parameter");
        build.flag("-Wno-deprecated-declarations");
        build.flag("-Wno-reorder-ctor");
    }

    build.compile("angelscript");

    // Build our C wrapper
    let mut wrapper_build = cc::Build::new();
    wrapper_build
        .cpp(true)
        .include("wrapper")
        .include("vendor/angelscript/sdk/angelscript/include")
        .include("vendor/angelscript/sdk/add_on/scriptstdstring")
        .include("vendor/angelscript/sdk/add_on/scriptarray")
        .include("vendor/angelscript/sdk/add_on/scriptdictionary")
        .flag_if_supported("-std=c++11");

    // Add all wrapper implementation files
    wrapper_build
        .file("wrapper/as_engine.cpp")
        .file("wrapper/as_module.cpp")
        .file("wrapper/as_context.cpp")
        .file("wrapper/as_function.cpp")
        .file("wrapper/as_typeinfo.cpp")
        .file("wrapper/as_generic.cpp")
        .file("wrapper/as_stringfactory.cpp")
        .file("wrapper/as_scriptobject.cpp");

    if target_os == "macos" {
        wrapper_build.flag("-mmacosx-version-min=10.15");
        wrapper_build.flag("-Wno-unused-parameter");
        wrapper_build.flag("-Wno-mismatched-tags");
    }

    wrapper_build.compile("angelscript_c");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header("wrapper/angelscript_c.h")
        // Include paths
        .clang_arg("-I./wrapper")
        .clang_arg("-I./vendor/angelscript/sdk/angelscript/include")
        .clang_arg("-I./vendor/angelscript/sdk/add_on/scriptstdstring")
        .clang_arg("-xc++")
        .clang_arg("-std=c++11")
        .impl_debug(true)
        .impl_partialeq(true)
        .wrap_unsafe_ops(true)
        // Whitelist what we want
        .allowlist_function("as.*")
        .allowlist_type("as.*")
        .allowlist_var("as.*")
        .rustified_enum("as.*")

        .allowlist_var("ANGELSCRIPT_VERSION")
        // Opaque types for C++ classes
        .opaque_type("asIScriptEngine")
        .opaque_type("asIScriptModule")
        .opaque_type("asIScriptContext")
        .opaque_type("asIScriptGeneric")
        .opaque_type("asIScriptObject")
        .opaque_type("asIScriptFunction")
        .opaque_type("asITypeInfo")
        .opaque_type("asIBinaryStream")
        .opaque_type("asIJITCompiler")
        .opaque_type("asIThreadManager")
        .opaque_type("asILockableSharedBool")
        .opaque_type("asIStringFactory")
        // Use core types
        .use_core()
        .derive_default(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper/*.*");
}
