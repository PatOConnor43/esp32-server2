fn main() -> Result<(), Box<dyn std::error::Error>> {
    embuild::espidf::sysenv::output();
    Ok(())
}
