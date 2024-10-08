fn main() {
    println!("cargo::rustc-link-arg=-Trom_coexist.x");
    println!("cargo::rustc-link-arg=-Trom_functions.x");
    println!("cargo::rustc-link-arg=-Trom_phy.x");
}
