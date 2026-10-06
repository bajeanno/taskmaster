fn main() {
    cc::Build::new()
        .file("src/daemon/signal_handling/interface.c")
        .compile("signal_handling_binary_interface");
}
