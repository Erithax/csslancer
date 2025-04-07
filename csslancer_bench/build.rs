use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    println!("cargo:rustc-env=TTARGET={}", target); // Use TTARGET to avoid shadowing
    println!("cargo:rustc-env=HHOST={}", env::var("HOST").unwrap());
}