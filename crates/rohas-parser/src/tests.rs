#[cfg(test)]
mod integration_tests {
    use crate::Parser;

    #[test]
    fn test_full_schema() {
        let input = r#"
            model User {
                id Int @id @auto
                name String
                email String @unique
                createdAt DateTime @default(now)
            }

            input CreateUserInput {
                name: String
                email: String
            }

            api CreateUser {
                method: POST
                path: "/users"
                body: CreateUserInput
                response: User
                triggers: [UserCreated]
            }

            event UserCreated {
                payload: User
                handler: [send_welcome_email, update_analytics]
            }

            cron CleanupOldUsers {
                schedule: "0 0 * * *"
                triggers: [UserDeleted]
            }
        "#;

        let schema = Parser::parse_string(input).expect("Failed to parse full schema");

        assert_eq!(schema.models.len(), 1);
        assert_eq!(schema.inputs.len(), 1);
        assert_eq!(schema.apis.len(), 1);
        assert_eq!(schema.events.len(), 1);
        assert_eq!(schema.crons.len(), 1);

        // Validate model
        let user_model = &schema.models[0];
        assert_eq!(user_model.name, "User");
        assert_eq!(user_model.fields.len(), 4);

        // Validate API
        let create_user_api = &schema.apis[0];
        assert_eq!(create_user_api.name, "CreateUser");
        assert_eq!(create_user_api.path, "/users");
        assert_eq!(create_user_api.triggers.len(), 1);

        // Validate event
        let user_created_event = &schema.events[0];
        assert_eq!(user_created_event.name, "UserCreated");
        assert_eq!(user_created_event.handlers.len(), 2);

        // Validate cron
        let cleanup_cron = &schema.crons[0];
        assert_eq!(cleanup_cron.name, "CleanupOldUsers");
        assert_eq!(cleanup_cron.schedule, "0 0 * * *");
    }
}
