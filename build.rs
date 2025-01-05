fn main() {
    #[cfg(features="espidf")]
    embuild::espidf::sysenv::output();
}