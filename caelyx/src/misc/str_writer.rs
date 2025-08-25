pub struct StrWriter {
    pub write: fn(&str),
}

impl core::fmt::Write for StrWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write.call((s,));
        Ok(())
    }
}
