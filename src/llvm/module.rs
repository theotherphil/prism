//! A trivial wrapper type for an LLVM module

use std::{
    ffi::CStr,
    fs::File,
    io::Write,
    path::Path
};
use llvm_sys::{core::*, prelude::*};

pub struct Module {
    pub module: LLVMModuleRef
}

impl Module {
    pub fn new(module: LLVMModuleRef) -> Module {
        Module { module }
    }

    pub fn dump_to_stdout(&self) {
        unsafe {
            LLVMDumpModule(self.module);
        }
    }

    pub fn dump_to_string(&self) -> String {
        unsafe {
            let c_str = LLVMPrintModuleToString(self.module);
            CStr::from_ptr(c_str).to_string_lossy().to_string()
        }
    }

    pub fn dump_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(self.dump_to_string().as_bytes()).map(|_| ())
    }
}
