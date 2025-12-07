//! Proc macros for rohas-orm
//!
//! Provides derive macros for models, queries, and relationships

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, DataStruct, Fields, LitStr};

/// Derive macro for Model trait
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Model)]
/// #[table_name = "users"]
/// struct User {
///     #[primary_key]
///     id: i64,
///     name: String,
///     email: String,
/// }
/// ```
#[proc_macro_derive(Model, attributes(table_name, primary_key))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    // Extract table name from attributes
    let table_name = extract_table_name(&input);
    let primary_key = extract_primary_key(&input);
    
    let primary_key_str = LitStr::new(&primary_key.to_string(), primary_key.span());
    let expanded = quote! {
        impl rohas_orm::Model for #name {
            fn table_name() -> &'static str {
                #table_name
            }
            
            fn primary_key() -> &'static str {
                #primary_key_str
            }
            
            fn primary_key_value(&self) -> rohas_orm::Result<Box<dyn std::any::Any + Send>> {
                Ok(Box::new(self.#primary_key.clone()))
            }
            
            async fn find_by_id(db: &rohas_orm::Database, id: i64) -> rohas_orm::Result<Option<Self>> {
                use rohas_orm::Query;
                let query = rohas_orm::QueryBuilder::select_all()
                    .from(Self::table_name())
                    .where_eq_num(Self::primary_key(), id)
                    .limit(1);
                
                let results = query.execute(db).await?;
                if results.is_empty() {
                    return Ok(None);
                }
                
                let model: Self = serde_json::from_value(results[0].clone())
                    .map_err(|e| rohas_orm::Error::Serialization(e))?;
                Ok(Some(model))
            }
            
            async fn find_all(db: &rohas_orm::Database) -> rohas_orm::Result<Vec<Self>> {
                use rohas_orm::Query;
                let query = rohas_orm::QueryBuilder::select_all()
                    .from(Self::table_name());
                
                let results = query.execute(db).await?;
                let models: Vec<Self> = results.into_iter()
                    .map(|v| serde_json::from_value(v).map_err(|e| rohas_orm::Error::Serialization(e)))
                    .collect::<rohas_orm::Result<Vec<_>>>()?;
                Ok(models)
            }
            
            async fn save(&self, db: &rohas_orm::Database) -> rohas_orm::Result<()> {
                use rohas_orm::Query;
                let pk_value = self.primary_key_value()?;
                let pk_num = pk_value.downcast_ref::<i64>()
                    .ok_or_else(|| rohas_orm::Error::Validation("Primary key must be i64".to_string()))?;
                
                // Check if exists
                if Self::find_by_id(db, *pk_num).await?.is_some() {
                    // Update
                    let json = serde_json::to_value(self)
                        .map_err(|e| rohas_orm::Error::Serialization(e))?;
                    let mut update = rohas_orm::QueryBuilder::update(Self::table_name());
                    
                    if let serde_json::Value::Object(map) = json {
                        for (key, value) in map {
                            if key != Self::primary_key() {
                                let val_str = match value {
                                    serde_json::Value::String(s) => s,
                                    serde_json::Value::Number(n) => n.to_string(),
                                    serde_json::Value::Bool(b) => b.to_string(),
                                    _ => value.to_string(),
                                };
                                update = update.set(&key, &val_str);
                            }
                        }
                    }
                    
                    update = update.where_eq_num(Self::primary_key(), *pk_num);
                    update.execute_affected(db).await?;
                } else {
                    // Insert
                    let json = serde_json::to_value(self)
                        .map_err(|e| rohas_orm::Error::Serialization(e))?;
                    
                    if let serde_json::Value::Object(map) = json {
                        let columns: Vec<String> = map.keys().cloned().collect();
                        let value_strings: Vec<String> = map.values().map(|v| {
                            match v {
                                serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
                                serde_json::Value::Number(n) => n.to_string(),
                                serde_json::Value::Bool(b) => b.to_string(),
                                _ => format!("'{}'", v.to_string().replace("'", "''")),
                            }
                        }).collect();
                        let values: Vec<&str> = value_strings.iter().map(|s| s.as_str()).collect();
                        
                        let insert = rohas_orm::QueryBuilder::insert(Self::table_name())
                            .values(values);
                        insert.execute_affected(db).await?;
                    }
                }
                
                Ok(())
            }
            
            async fn delete(&self, db: &rohas_orm::Database) -> rohas_orm::Result<()> {
                use rohas_orm::Query;
                let pk_value = self.primary_key_value()?;
                let pk_num = pk_value.downcast_ref::<i64>()
                    .ok_or_else(|| rohas_orm::Error::Validation("Primary key must be i64".to_string()))?;
                
                let delete = rohas_orm::QueryBuilder::delete(Self::table_name())
                    .where_eq_num(Self::primary_key(), *pk_num);
                delete.execute_affected(db).await?;
                Ok(())
            }
            
            async fn create(db: &rohas_orm::Database, data: Self) -> rohas_orm::Result<Self> {
                data.save(db).await?;
                Ok(data)
            }
            
            async fn update(db: &rohas_orm::Database, id: i64, data: Self) -> rohas_orm::Result<Self> {
                data.save(db).await?;
                Ok(data)
            }
        }
    };
    
    TokenStream::from(expanded)
}

fn extract_table_name(input: &DeriveInput) -> String {
    for attr in &input.attrs {
        if attr.path().is_ident("table_name") {
            if let Ok(meta) = attr.parse_args::<syn::LitStr>() {
                return meta.value();
            }
        }
    }
    let name = input.ident.to_string().to_lowercase();
    format!("{}s", name)
}

fn extract_primary_key(input: &DeriveInput) -> syn::Ident {
    if let Data::Struct(DataStruct { fields: Fields::Named(ref fields), .. }) = input.data {
        for field in &fields.named {
            for attr in &field.attrs {
                if attr.path().is_ident("primary_key") {
                    return field.ident.clone().unwrap();
                }
            }
        }
    }
    syn::parse_str("id").unwrap()
}

