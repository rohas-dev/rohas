use crate::error::{Error, Result};
use rohas_parser::ast::{FieldType, Model as ParserModel, Schema};
use rohas_parser::Parser;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Relationship {
    pub field_name: String,
    pub related_model: String,
    pub relationship_type: RelationshipType,
    pub foreign_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationshipType {
    OneToOne,
    OneToMany,
    ManyToMany,
    BelongsTo,
}

pub struct Codegen {
    pub output_dir: PathBuf,
    pub models: Vec<ParserModel>,
    pub model_names: HashSet<String>,
}

impl Codegen {
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            models: Vec::new(),
            model_names: HashSet::new(),
        }
    }

    pub fn load_schema_dir<P: AsRef<Path>>(&mut self, schema_dir: P) -> Result<()> {
        let schema_dir = schema_dir.as_ref();
        
        if !schema_dir.exists() {
            return Err(Error::Codegen(format!("Schema directory does not exist: {:?}", schema_dir)));
        }

        let mut schema = Schema::new();

        self.find_and_parse_ro_files(schema_dir, &mut schema)?;

        schema.validate()
            .map_err(|e| Error::Codegen(format!("Schema validation failed: {}", e)))?;

        self.models = schema.models;
        self.model_names = self.models.iter().map(|m| m.name.clone()).collect();

        Ok(())
    }

    pub fn load_schema_file<P: AsRef<Path>>(&mut self, schema_file: P) -> Result<()> {
        let schema = Parser::parse_file(schema_file)
            .map_err(|e| Error::Codegen(format!("Failed to parse schema file: {}", e)))?;

        schema.validate()
            .map_err(|e| Error::Codegen(format!("Schema validation failed: {}", e)))?;

        self.models = schema.models;
        self.model_names = self.models.iter().map(|m| m.name.clone()).collect();

        Ok(())
    }

    fn find_and_parse_ro_files(&self, dir: &Path, schema: &mut Schema) -> Result<()> {
        let entries = fs::read_dir(dir)
            .map_err(|e| Error::Codegen(format!("Failed to read directory {:?}: {}", dir, e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| Error::Codegen(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();

            if path.is_dir() {
                self.find_and_parse_ro_files(&path, schema)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("ro") {
                let file_schema = Parser::parse_file(&path)
                    .map_err(|e| Error::Codegen(format!("Failed to parse {:?}: {}", path, e)))?;
                
                schema.models.extend(file_schema.models);
                schema.apis.extend(file_schema.apis);
                schema.events.extend(file_schema.events);
                schema.crons.extend(file_schema.crons);
                schema.inputs.extend(file_schema.inputs);
                schema.websockets.extend(file_schema.websockets);
            }
        }

        Ok(())
    }

    pub fn generate_rust_models(&self) -> Result<()> {
        std::fs::create_dir_all(&self.output_dir)
            .map_err(|e| Error::Codegen(format!("Failed to create output directory: {}", e)))?;

        let mut mod_rs = String::new();
        mod_rs.push_str("// Auto-generated models - DO NOT EDIT\n\n");

        for model in &self.models {
            let rust_code = self.generate_rust_model(model)?;
            let file_path = self.output_dir.join(format!("{}.rs", model.name.to_lowercase()));
            
            std::fs::write(&file_path, rust_code)
                .map_err(|e| Error::Codegen(format!("Failed to write file {:?}: {}", file_path, e)))?;
            
            mod_rs.push_str(&format!("pub mod {};\n", model.name.to_lowercase()));
        }

        std::fs::write(self.output_dir.join("mod.rs"), mod_rs)
            .map_err(|e| Error::Codegen(format!("Failed to write mod.rs: {}", e)))?;

        Ok(())
    }

    pub fn generate_python_models(&self) -> Result<()> {
        std::fs::create_dir_all(&self.output_dir)
            .map_err(|e| Error::Codegen(format!("Failed to create output directory: {}", e)))?;

        // Generate __init__.py
        let mut init_py = String::new();
        init_py.push_str("# Auto-generated models - DO NOT EDIT\n\n");

        for model in &self.models {
            let python_code = self.generate_python_model(model)?;
            let file_path = self.output_dir.join(format!("{}.py", model.name.to_lowercase()));
            
            std::fs::write(&file_path, python_code)
                .map_err(|e| Error::Codegen(format!("Failed to write file {:?}: {}", file_path, e)))?;
            
            init_py.push_str(&format!("from .{} import {}\n", model.name.to_lowercase(), model.name));
        }

        init_py.push_str("\n__all__ = [\n");
        for model in &self.models {
            init_py.push_str(&format!("    '{}',\n", model.name));
        }
        init_py.push_str("]\n");

        std::fs::write(self.output_dir.join("__init__.py"), init_py)
            .map_err(|e| Error::Codegen(format!("Failed to write __init__.py: {}", e)))?;

        Ok(())
    }

    /// Detect relationships for a model
    fn detect_relationships(&self, model: &ParserModel) -> Vec<Relationship> {
        let mut relationships = Vec::new();

        for field in &model.fields {
            if let FieldType::Custom(ref model_name) = field.field_type {
                if self.model_names.contains(model_name) {
                    // Check for relationship attributes
                    let relation_attr = field.attributes.iter().find(|a| a.name == "relation");
                    let one_to_one_attr = field.attributes.iter().find(|a| a.name == "oneToOne");
                    let one_to_many_attr = field.attributes.iter().find(|a| a.name == "oneToMany");
                    let many_to_many_attr = field.attributes.iter().find(|a| a.name == "manyToMany");
                    
                    let foreign_key = relation_attr
                        .and_then(|a| a.args.first())
                        .cloned();

                    // Determine relationship type based on attributes or field type
                    let rel_type = if many_to_many_attr.is_some() {
                        RelationshipType::ManyToMany
                    } else if one_to_many_attr.is_some() {
                        RelationshipType::OneToMany
                    } else if one_to_one_attr.is_some() {
                        RelationshipType::OneToOne
                    } else if field.optional {
                        RelationshipType::BelongsTo
                    } else {
                        RelationshipType::OneToOne
                    };

                    relationships.push(Relationship {
                        field_name: field.name.clone(),
                        related_model: model_name.clone(),
                        relationship_type: rel_type,
                        foreign_key,
                    });
                }
            } else if let FieldType::Array(inner) = &field.field_type {
                if let FieldType::Custom(ref model_name) = **inner {
                    if self.model_names.contains(model_name) {
                        // Check for explicit relationship type
                        let many_to_many_attr = field.attributes.iter().find(|a| a.name == "manyToMany");
                        let one_to_many_attr = field.attributes.iter().find(|a| a.name == "oneToMany");
                        
                        let rel_type = if many_to_many_attr.is_some() {
                            RelationshipType::ManyToMany
                        } else if one_to_many_attr.is_some() {
                            RelationshipType::OneToMany
                        } else {
                            RelationshipType::ManyToMany // Default for arrays
                        };
                        
                        relationships.push(Relationship {
                            field_name: field.name.clone(),
                            related_model: model_name.clone(),
                            relationship_type: rel_type,
                            foreign_key: None,
                        });
                    }
                }
            }
        }

        relationships
    }

    fn generate_rust_model(&self, model: &ParserModel) -> Result<String> {
        let mut code = String::new();
        
        code.push_str("// Auto-generated - DO NOT EDIT\n");
        code.push_str("use rohas_orm::prelude::*;\n");
        code.push_str("use serde::{Deserialize, Serialize};\n");
        code.push_str("use chrono::{DateTime, Utc};\n\n");

        // Generate relationships
        let relationships = self.detect_relationships(model);
        if !relationships.is_empty() {
            for rel in &relationships {
                code.push_str(&format!("use super::{};\n", rel.related_model.to_lowercase()));
            }
            code.push_str("\n");
        }
        
        code.push_str("#[derive(Model, Debug, Clone, Serialize, Deserialize)]\n");
        code.push_str(&format!("#[table_name = \"{}\"]\n", model.name.to_lowercase()));
        code.push_str(&format!("pub struct {} {{\n", model.name));
        
        let mut has_primary_key = false;
        for field in &model.fields {
            // Handle attributes
            let is_primary = field.attributes.iter().any(|attr| attr.name == "id");
            let is_unique = field.attributes.iter().any(|attr| attr.name == "unique");
            let has_default = field.attributes.iter().find(|attr| attr.name == "default");
            let is_auto = field.attributes.iter().any(|attr| attr.name == "auto");

            if is_primary && !has_primary_key {
                code.push_str("    #[primary_key]\n");
                has_primary_key = true;
            }

            let mut rust_type = if let FieldType::Custom(ref model_name) = field.field_type {
                if self.model_names.contains(model_name) {
                    if field.optional {
                        format!("Option<{}>", model_name)
                    } else {
                        model_name.clone()
                    }
                } else {
                    self.field_type_to_rust(&field.field_type)
                }
            } else if let FieldType::Array(inner) = &field.field_type {
                if let FieldType::Custom(ref model_name) = **inner {
                    if self.model_names.contains(model_name) {
                        format!("Vec<{}>", model_name)
                    } else {
                        self.field_type_to_rust(&field.field_type)
                    }
                } else {
                    self.field_type_to_rust(&field.field_type)
                }
            } else {
                self.field_type_to_rust(&field.field_type)
            };

            if field.optional && !rust_type.starts_with("Option<") && !rust_type.starts_with("Vec<") {
                rust_type = format!("Option<{}>", rust_type);
            }

            if let Some(default) = has_default {
                if let Some(default_value) = default.args.first() {
                    match default_value.as_str() {
                        "now" => {
                            code.push_str("    #[serde(default)]\n");
                        }
                        _ => {
                            // For other defaults, we'll handle them in the struct initialization
                        }
                    }
                }
            }

            code.push_str(&format!("    pub {}: {},\n", field.name, rust_type));
        }
        
        code.push_str("}\n\n");

        // Generate relationship methods
        for rel in &relationships {
            match rel.relationship_type {
                RelationshipType::BelongsTo | RelationshipType::OneToOne => {
                    code.push_str(&format!(
                        "impl {} {{\n",
                        model.name
                    ));
                    code.push_str(&format!(
                        "    pub async fn load_{}(&self, db: &Database) -> Result<Option<{}>> {{\n",
                        rel.field_name, rel.related_model
                    ));
                    if let Some(ref fk) = rel.foreign_key {
                    code.push_str(&format!(
                        "        if let Some(fk_value) = self.{} {{\n",
                        fk
                    ));
                    code.push_str(&format!(
                        "            {}::find_by_id(db, fk_value).await\n",
                        rel.related_model
                    ));
                    code.push_str("        } else {\n");
                    code.push_str("            Ok(None)\n");
                    code.push_str("        }\n");
                    } else {
                        code.push_str(&format!(
                            "        if let Some(fk_value) = self.{} {{\n",
                            rel.field_name
                        ));
                        code.push_str(&format!(
                            "            {}::find_by_id(db, fk_value).await\n",
                            rel.related_model
                        ));
                        code.push_str("        } else {\n");
                        code.push_str("            Ok(None)\n");
                        code.push_str("        }\n");
                    }
                    code.push_str("    }\n");
                    code.push_str("}\n\n");
                }
                RelationshipType::OneToMany => {
                    code.push_str(&format!(
                        "impl {} {{\n",
                        model.name
                    ));
                    code.push_str(&format!(
                        "    pub async fn load_{}(&self, db: &Database) -> Result<Vec<{}>> {{\n",
                        rel.field_name, rel.related_model
                    ));
                    code.push_str(&format!(
                        "        use rohas_orm::Query;\n",
                    ));
                    code.push_str(&format!(
                        "        let query = QueryBuilder::select_all()\n"
                    ));
                    code.push_str(&format!(
                        "            .from(\"{}\")\n",
                        rel.related_model.to_lowercase()
                    ));
                    code.push_str(&format!(
                        "            .where_eq_num(\"{}\", self.id);\n",
                        format!("{}Id", model.name.to_lowercase())
                    ));
                    code.push_str("        let results = query.execute(db).await?;\n");
                    code.push_str(&format!(
                        "        results.into_iter()\n"
                    ));
                    code.push_str(&format!(
                        "            .map(|v| serde_json::from_value(v).map_err(|e| Error::Serialization(e)))\n"
                    ));
                    code.push_str(&format!(
                        "            .collect::<Result<Vec<{}>>>()\n",
                        rel.related_model
                    ));
                    code.push_str("    }\n");
                    code.push_str("}\n\n");
                }
                RelationshipType::ManyToMany => {
                    // Many-to-many relationships need a join table
                    code.push_str(&format!(
                        "impl {} {{\n",
                        model.name
                    ));
                    code.push_str(&format!(
                        "    pub async fn load_{}(&self, db: &Database) -> Result<Vec<{}>> {{\n",
                        rel.field_name, rel.related_model
                    ));
                    code.push_str("        // Many-to-many relationship - implement join table query\n");
                    code.push_str("        todo!(\"Many-to-many relationships require join table implementation\")\n");
                    code.push_str("    }\n");
                    code.push_str("}\n\n");
                }
            }
        }
        
        Ok(code)
    }

    fn generate_python_model(&self, model: &ParserModel) -> Result<String> {
        let mut code = String::new();
        
        code.push_str("# Auto-generated - DO NOT EDIT\n");
        code.push_str("from rohas_orm import Model, Field, Database, Table, Index, Unique\n");
        code.push_str("from datetime import datetime\n");
        code.push_str("from typing import Optional, List\n\n");

        let relationships = self.detect_relationships(model);
        if !relationships.is_empty() {
            for rel in &relationships {
                code.push_str(&format!("from .{} import {}\n", rel.related_model.to_lowercase(), rel.related_model));
            }
            code.push_str("\n");
        }
        
        let mut decorators = Vec::new();
        
        let table_name = self.get_table_name(model);
        decorators.push(format!("@Table(name=\"{}\")", table_name));
        
        let unique_fields: Vec<String> = model.fields
            .iter()
            .filter(|f| f.attributes.iter().any(|attr| attr.name == "unique"))
            .map(|f| f.name.clone())
            .collect();
        
        if !unique_fields.is_empty() {
            let fields_str = unique_fields.iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(", ");
            decorators.push(format!("@Unique(fields=[{}])", fields_str));
        }
        
        let index_fields: Vec<String> = model.fields
            .iter()
            .filter(|f| f.attributes.iter().any(|attr| attr.name == "index"))
            .map(|f| f.name.clone())
            .collect();
        
        if !index_fields.is_empty() {
            let fields_str = index_fields.iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(", ");
            decorators.push(format!("@Index(fields=[{}])", fields_str));
        }
        
        for decorator in &decorators {
            code.push_str(&format!("{}\n", decorator));
        }
        
        code.push_str(&format!("class {}(Model):\n", model.name));
        code.push_str(&format!("    \"\"\"Generated model for {}\"\"\"\n\n", model.name));
        
        let mut has_primary_key = false;
        for field in &model.fields {
            let is_primary = field.attributes.iter().any(|attr| attr.name == "id");
            let is_unique = field.attributes.iter().any(|attr| attr.name == "unique");
            let has_default = field.attributes.iter().find(|attr| attr.name == "default");
            let is_auto = field.attributes.iter().any(|attr| attr.name == "auto");

            let mut python_type = if let FieldType::Custom(ref model_name) = field.field_type {
                if self.model_names.contains(model_name) {
                    if field.optional {
                        format!("Optional[{}]", model_name)
                    } else {
                        model_name.clone()
                    }
                } else {
                    self.field_type_to_python(&field.field_type)
                }
            } else if let FieldType::Array(inner) = &field.field_type {
                if let FieldType::Custom(ref model_name) = **inner {
                    if self.model_names.contains(model_name) {
                        format!("List[{}]", model_name)
                    } else {
                        self.field_type_to_python(&field.field_type)
                    }
                } else {
                    self.field_type_to_python(&field.field_type)
                }
            } else {
                self.field_type_to_python(&field.field_type)
            };

            if field.optional && !python_type.starts_with("Optional[") && !python_type.starts_with("List[") {
                python_type = format!("Optional[{}]", python_type);
            }

            let mut field_attrs = Vec::<String>::new();
            if is_primary && !has_primary_key {
                field_attrs.push("primary_key=True".to_string());
                has_primary_key = true;
            }
            if is_unique {
                field_attrs.push("unique=True".to_string());
            }
            if let Some(default) = has_default {
                if let Some(default_value) = default.args.first() {
                    if default_value == "now" {
                        field_attrs.push("default=datetime.now".to_string());
                    } else {
                        field_attrs.push(format!("default=\"{}\"", default_value));
                    }
                }
            }

            let field_def = if !field_attrs.is_empty() {
                format!("{}: {} = Field({})", field.name, python_type, field_attrs.join(", "))
            } else {
                format!("{}: {}", field.name, python_type)
            };
            code.push_str(&format!("    {}\n", field_def));
        }

        if !relationships.is_empty() {
            code.push_str("\n");
            for rel in &relationships {
                match rel.relationship_type {
                    RelationshipType::BelongsTo | RelationshipType::OneToOne => {
                        code.push_str(&format!(
                            "    async def load_{}(self, db: Database) -> Optional[{}]:\n",
                            rel.field_name, rel.related_model
                        ));
                        if let Some(ref fk) = rel.foreign_key {
                            code.push_str(&format!(
                                "        return await {}.find_by_id(db, getattr(self, '{}'))\n",
                                rel.related_model, fk
                            ));
                        } else {
                            code.push_str(&format!(
                                "        return await {}.find_by_id(db, getattr(self, '{}'))\n",
                                rel.related_model, rel.field_name
                            ));
                        }
                    }
                    RelationshipType::OneToMany => {
                        code.push_str(&format!(
                            "    async def load_{}(self, db: Database) -> List[{}]:\n",
                            rel.field_name, rel.related_model
                        ));
                        code.push_str("        from rohas_orm import QueryBuilder\n");
                        code.push_str(&format!(
                            "        query = QueryBuilder.select_all()\\\n"
                        ));
                        code.push_str(&format!(
                            "            .from_(\"{}\")\\\n",
                            rel.related_model.to_lowercase()
                        ));
                        code.push_str(&format!(
                            "            .where_eq_num(\"{}\", self.id)\n",
                            format!("{}Id", model.name.to_lowercase())
                        ));
                        code.push_str("        return await db.query(query)\n");
                    }
                    RelationshipType::ManyToMany => {
                        code.push_str(&format!(
                            "    async def load_{}(self, db: Database) -> List[{}]:\n",
                            rel.field_name, rel.related_model
                        ));
                        code.push_str("        # Many-to-many relationship - implement join table query\n");
                        code.push_str("        raise NotImplementedError(\"Many-to-many relationships require join table implementation\")\n");
                    }
                }
            }
        }
        
        Ok(code)
    }

    fn field_type_to_rust(&self, field_type: &FieldType) -> String {
        match field_type {
            FieldType::String => "String".to_string(),
            FieldType::Int => "i64".to_string(),
            FieldType::Float => "f64".to_string(),
            FieldType::Boolean => "bool".to_string(),
            FieldType::DateTime => "DateTime<Utc>".to_string(),
            FieldType::Json => "serde_json::Value".to_string(),
            FieldType::Array(inner) => {
                format!("Vec<{}>", self.field_type_to_rust(inner))
            }
            FieldType::Custom(name) => name.clone(),
        }
    }

    fn field_type_to_python(&self, field_type: &FieldType) -> String {
        match field_type {
            FieldType::String => "str".to_string(),
            FieldType::Int => "int".to_string(),
            FieldType::Float => "float".to_string(),
            FieldType::Boolean => "bool".to_string(),
            FieldType::DateTime => "datetime".to_string(),
            FieldType::Json => "dict".to_string(),
            FieldType::Array(inner) => {
                format!("list[{}]", self.field_type_to_python(inner))
            }
            FieldType::Custom(name) => name.clone(),
        }
    }
    
    fn get_table_name(&self, model: &ParserModel) -> String {
        if let Some(table_attr) = model.attributes.iter().find(|attr| attr.name == "table") {
            if let Some(table_name) = table_attr.args.first() {
                return table_name.clone();
            }
        }
        
        format!("{}s", model.name.to_lowercase())
    }
}
