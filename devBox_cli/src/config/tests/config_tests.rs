use config::yaml_parser::ProjectConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_valid_config() {
        let yaml_content = r#"
        name: "test-app"
        description: "test: should succeed"
        commands:
            start:
                dev: "npm run dev",
                build: "npm run build"
        "#;
        let config: ProjectConfig = serde_yaml::from_str(yaml_content).unwrap();

        assert_eq!(config.name, "test-app");
        assert_eq!(config.commands.start.dev, "npm run dev")
        // TODO: add more configs in the yaml to make sure all is available
    }
    
    #[test]
    fn test_invalid_config_fails() {
        let invalid_yaml = "name: 123";

        let result = serde_yaml::from_str::<ProjectConfig>(invalid_yaml);
        assert!(result.is_err());
        // TODO: try more errors that is possible
    }
}