use std::env;
fn main() {
    let target = env::var("CARGO_CFG_TARGET_OS");
    if target == Ok("macos".to_string()) {
        unsafe {
	    env::set_var("PKG_CONFIG_PATH",
	        "/usr/local/lib/pkgconfig:/Library/Frameworks/GStreamer.framework/Libraries/pkgconfig"
	    );
	    let lib = "/Library/Frameworks/GStreamer.framework/Libraries";
	    env::set_var("GST_PLUGIN_PATH", lib);
	    env::set_var("DYLD_FALLBACK_LIBRARY_PATH", lib);
	}
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=11.0");
        println!("cargo:rustc-link-search=framework=/Library/Frameworks");
        println!("cargo:rustc-link-arg=-Wl,-headerpad_max_install_names,-rpath,/Library/Frameworks/GStreamer.framework/Libraries");
    }
}
