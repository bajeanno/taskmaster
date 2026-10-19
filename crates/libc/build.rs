fn main() {
    cc::Build::new()
        .file("src/sys/wifexited.c")
        .compile("wifexited_macro_interface");
}
