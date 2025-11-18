use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "rohas.pest"]
pub struct RohasParser;

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;

    #[test]
    fn test_basic_parsing() {
        let input = r#"
            model User {
                id Int @id @auto
                name String
            }
        "#;

        let result = RohasParser::parse(Rule::schema, input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }
}
