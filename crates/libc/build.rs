fn main() {
    cc::Build::new()
        .file("src/sys/wait_macros.c")
        .compile("wifexited_macro_interface");
}
