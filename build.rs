fn main() {
    #[cfg(feature = "esp32-examples")]
    embuild::espidf::sysenv::output();
}