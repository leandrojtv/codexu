pub struct Reporter;

impl Reporter {
    pub fn summarize(items: &[String]) -> String {
        items.join("\n")
    }
}
