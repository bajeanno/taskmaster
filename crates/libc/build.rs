fn main() {
    cc::Build::new()
        .file("src/sys/wait_macros.c")
        .compile("wait_macros_interfaces");
}
