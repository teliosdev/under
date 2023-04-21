use rustc_version::{version, version_meta, Channel};

fn main() {
    assert!(version().unwrap().major >= 1);
    if let Channel::Nightly = version_meta().unwrap().channel {
        println!("cargo:rustc-cfg=nightly");
    }
}
