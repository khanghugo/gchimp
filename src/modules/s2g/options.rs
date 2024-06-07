#[derive(Clone)]
pub struct S2GOptions {
    /// Proceeds even when there is failure
    pub force: bool,
    /// Adds "_goldsrc" to the output model name
    ///
    /// This might overwrite original model
    pub add_suffix: bool,
    /// Ignores converted models that have "_goldsrc" suffix.
    pub ignore_converted: bool,
    /// Mark the texture with flat shade flag
    pub flatshade: bool,
}

impl Default for S2GOptions {
    fn default() -> Self {
        Self {
            force: false,
            add_suffix: true,
            ignore_converted: true,
            flatshade: true,
        }
    }
}
