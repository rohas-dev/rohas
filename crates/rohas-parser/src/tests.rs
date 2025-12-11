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

    #[test]
    fn test_relations() {
        let input = r#"
            model User {
                id Int @id @auto
                name String
                email String @unique
                posts Post[] @relation({name: "UserPosts"})
                profile Profile? @relation({name: "UserProfile"})
            }

            model Post {
                id Int @id @auto
                title String
                content String?
                published Boolean @default(false)
                userId Int
                author User? @relation({name: "UserPosts", fields: [userId], references: [id], onDelete: Cascade, onUpdate: Cascade})
                tags Tag[] @relation({name: "PostTags", through: PostTag})
            }

            model Profile {
                id Int @id @auto
                bio String?
                userId Int @unique
                user User? @relation({name: "UserProfile", fields: [userId], references: [id], onDelete: Cascade})
            }

            model Tag {
                id Int @id @auto
                name String @unique
                posts Post[] @relation({name: "PostTags", through: PostTag})
            }

            model PostTag {
                id Int @id @auto
                postId Int
                tagId Int
                post Post @relation({fields: [postId], references: [id], onDelete: Cascade})
                tag Tag @relation({fields: [tagId], references: [id], onDelete: Cascade})
                
                @@unique([postId, tagId])
            }
        "#;

        let schema = Parser::parse_string(input).expect("Failed to parse relations schema");

        assert_eq!(schema.models.len(), 5);

        let user_model = &schema.models[0];
        assert_eq!(user_model.name, "User");
        assert_eq!(user_model.fields.len(), 5);
        
        let posts_field = &user_model.fields[3];
        assert_eq!(posts_field.name, "posts");
        assert_eq!(posts_field.attributes.len(), 1);
        let posts_relation = &posts_field.attributes[0];
        assert_eq!(posts_relation.name, "relation");
        assert!(posts_relation.relation_config.is_some());
        let posts_config = posts_relation.relation_config.as_ref().unwrap();
        assert_eq!(posts_config.name.as_ref().unwrap(), "UserPosts");

        let post_model = &schema.models[1];
        assert_eq!(post_model.name, "Post");
        
        let author_field = &post_model.fields[5];
        assert_eq!(author_field.name, "author");
        let author_relation = &author_field.attributes[0];
        assert!(author_relation.relation_config.is_some());
        let author_config = author_relation.relation_config.as_ref().unwrap();
        assert_eq!(author_config.name.as_ref().unwrap(), "UserPosts");
        assert_eq!(author_config.fields.as_ref().unwrap(), &vec!["userId".to_string()]);
        assert_eq!(author_config.references.as_ref().unwrap(), &vec!["id".to_string()]);
        assert!(matches!(author_config.on_delete, Some(crate::ast::ReferentialAction::Cascade)));
        assert!(matches!(author_config.on_update, Some(crate::ast::ReferentialAction::Cascade)));

        let tags_field = &post_model.fields[6];
        assert_eq!(tags_field.name, "tags");
        let tags_relation = &tags_field.attributes[0];
        assert!(tags_relation.relation_config.is_some());
        let tags_config = tags_relation.relation_config.as_ref().unwrap();
        assert_eq!(tags_config.name.as_ref().unwrap(), "PostTags");
        assert_eq!(tags_config.through.as_ref().unwrap(), "PostTag");

        let post_tag_model = &schema.models[4];
        assert_eq!(post_tag_model.name, "PostTag");
        assert_eq!(post_tag_model.fields.len(), 5);
        
        assert_eq!(post_tag_model.attributes.len(), 1);
        let unique_attr = &post_tag_model.attributes[0];
        assert_eq!(unique_attr.name, "unique");
        assert_eq!(unique_attr.args.len(), 1);
        assert!(unique_attr.args[0].contains("postId"));
        assert!(unique_attr.args[0].contains("tagId"));
    }

    #[test]
    fn test_model_attributes() {
        let input = r#"
            model User {
                id Int @id @auto
                email String
                name String
                
                @@unique([email])
                @@index([name, email])
            }
        "#;

        let schema = Parser::parse_string(input).expect("Failed to parse model attributes");
        
        assert_eq!(schema.models.len(), 1);
        let user_model = &schema.models[0];
        
        assert_eq!(user_model.attributes.len(), 2);
        
        let unique_attr = &user_model.attributes[0];
        assert_eq!(unique_attr.name, "unique");
        assert_eq!(unique_attr.args.len(), 1);
        
        let index_attr = &user_model.attributes[1];
        assert_eq!(index_attr.name, "index");
        assert_eq!(index_attr.args.len(), 1);
    }
    
    #[test]
    fn test_comprehensive_relations() {
        let content = std::fs::read_to_string(
            "./src/relations_example.ro"
        ).unwrap();
        
        let schema = Parser::parse_string(&content).expect("Failed to parse comprehensive relations");
        
        println!("Models parsed: {}", schema.models.len());
        for model in &schema.models {
            println!("  - {} ({} fields, {} model attributes)", 
                model.name, 
                model.fields.len(),
                model.attributes.len()
            );
        }
        
        assert!(schema.models.len() >= 6);
    }
}
