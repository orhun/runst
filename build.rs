// use dbus_codegen::{ConnectionType, GenOpts, ServerAccess};
// use std::env;
// use std::error::Error;
// use std::fs;
// use std::path::Path;

// const INTROSPECTION_PATH: &str = "dbus/introspection.xml";
// -> Result<(), Box<dyn Error>>
fn main() {
    // let introspection = fs::read_to_string(INTROSPECTION_PATH)?;
    // let out_dir = env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?;

    // let gen_path = Path::new(&out_dir).join("introspection.rs");
    // let gen_opts = GenOpts {
    //     methodtype: None,
    //     crossroads: true,
    //     skipprefix: None,
    //     serveraccess: ServerAccess::RefClosure,
    //     genericvariant: false,
    //     connectiontype: ConnectionType::Blocking,
    //     propnewtype: false,
    //     interfaces: None,
    //     ..Default::default()
    // };

    // let code = dbus_codegen::generate(&introspection, &gen_opts)?;
    // fs::write(&gen_path, code)?;

    // println!("D-Bus code generated at {gen_path:?}");
    // println!("cargo:rerun-if-changed={INTROSPECTION_PATH}");
    // println!("cargo:rerun-if-changed=build.rs");
    // Ok(())
}
