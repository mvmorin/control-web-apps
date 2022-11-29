// Setup some basic print functions
#[cfg(target_arch = "wasm32")]
pub mod wasm_print {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console)]
        fn log(s: &str);
    }

    pub fn basic_print(s: &str) {
        log(s);
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_print::basic_print;

#[cfg(not(target_arch = "wasm32"))]
pub mod stdio_print {
    pub fn basic_print(s: &str) {
        println!("{}", s);
        // should probably find a way to directly write a string slice to stdout...
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use stdio_print::basic_print;
