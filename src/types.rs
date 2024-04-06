pub trait Cli {
    fn name(&self) -> &'static str;
    /// `args[1]` is the name of the function.
    ///
    /// Arguments for the the functions start at `args[2]`
    fn cli(&self);
    fn cli_help(&self);
}
