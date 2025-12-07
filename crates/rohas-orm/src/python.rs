//! Python bindings for rohas-orm using PyO3

use crate::connection::Database;
use crate::error::{Error, Result};
use crate::query::Query;
use crate::query_builder::QueryBuilder;
use pyo3::exceptions::{PyConnectionError, PyRuntimeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule, PyString, PyFloat, PyInt, PyBool, PyType};
use std::sync::Arc;

#[pymodule]
#[pyo3(name = "rohas_orm")]
fn rohas_orm(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDatabase>()?;
    m.add_class::<PyQueryBuilder>()?;
    m.add_class::<PyField>()?;
    m.add_class::<PyTable>()?;
    m.add_class::<PyIndex>()?;
    m.add_class::<PyUnique>()?;
    m.add_function(wrap_pyfunction!(connect, m)?)?;
    Ok(())
}

/// Python wrapper for Database
#[pyclass]
pub struct PyDatabase {
    db: Arc<Database>,
}

#[pymethods]
impl PyDatabase {
    #[new]
    fn new(url: &str) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<PyRuntimeError, _>(format!("Failed to create runtime: {}", e)))?;
        
        let db = rt.block_on(Database::connect(url))
            .map_err(|e| PyErr::new::<PyConnectionError, _>(format!("{}", e)))?;
        
        Ok(Self {
            db: Arc::new(db),
        })
    }

    #[classmethod]
    fn connect(_cls: &Bound<'_, PyType>, url: &str) -> PyResult<Self> {
        Self::new(url)
    }

    fn execute(&self, query: &str) -> PyResult<u64> {
        let db = self.db.clone();
        let query = query.to_string();
        
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<PyRuntimeError, _>(format!("Failed to create runtime: {}", e)))?;
        
        let result = rt.block_on(db.execute(&query))
            .map_err(|e| PyErr::new::<PyRuntimeError, _>(format!("{}", e)))?;
        
        Ok(result)
    }

    fn query(&self, query: &PyQueryBuilder, py: Python<'_>) -> PyResult<Py<PyList>> {
        let db = self.db.clone();
        let query_builder = query.builder.clone();
        
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<PyRuntimeError, _>(format!("Failed to create runtime: {}", e)))?;
        
        let results = rt.block_on(async {
            query_builder.execute(&db).await
        })
        .map_err(|e| PyErr::new::<PyRuntimeError, _>(format!("{}", e)))?;
        
        let list = PyList::empty(py);
        for result in results {
            if let serde_json::Value::Object(map) = result {
                let dict = PyDict::new(py);
                for (key, value) in map {
                    let py_value: PyObject = match value {
                        serde_json::Value::String(s) => PyString::new(py, &s).into(),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                PyInt::new(py, i).into()
                            } else if let Some(f) = n.as_f64() {
                                PyFloat::new(py, f).into()
                            } else {
                                PyString::new(py, &n.to_string()).into()
                            }
                        }
                        serde_json::Value::Bool(b) => {
                            // Convert bool to int (0 or 1) for compatibility
                            PyInt::new(py, if b { 1 } else { 0 }).into()
                        }
                        serde_json::Value::Null => py.None().into(),
                        _ => PyString::new(py, &value.to_string()).into(),
                    };
                    dict.set_item(key, py_value)?;
                }
                list.append(dict)?;
            }
        }
        Ok(list.into())
    }
}

/// Python wrapper for QueryBuilder
#[pyclass]
pub struct PyQueryBuilder {
    builder: QueryBuilder,
}

#[pymethods]
impl PyQueryBuilder {
    #[new]
    fn new() -> Self {
        Self {
            builder: QueryBuilder::select_all(),
        }
    }

    #[staticmethod]
    fn select(columns: Vec<String>) -> Self {
        let cols: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
        Self {
            builder: QueryBuilder::select(&cols),
        }
    }

    #[staticmethod]
    fn select_all() -> Self {
        Self {
            builder: QueryBuilder::select_all(),
        }
    }

    fn from_(&mut self, table: &str) -> PyResult<()> {
        self.builder = self.builder.clone().from(table);
        Ok(())
    }

    fn where_eq(&mut self, column: &str, value: &str) -> PyResult<()> {
        self.builder = self.builder.clone().where_eq(column, value);
        Ok(())
    }

    fn where_eq_num(&mut self, column: &str, value: i64) -> PyResult<()> {
        self.builder = self.builder.clone().where_eq_num(column, value);
        Ok(())
    }

    fn order_by(&mut self, column: &str, direction: &str) -> PyResult<()> {
        self.builder = self.builder.clone().order_by(column, direction);
        Ok(())
    }

    fn limit(&mut self, limit: u64) -> PyResult<()> {
        self.builder = self.builder.clone().limit(limit);
        Ok(())
    }

    fn offset(&mut self, offset: u64) -> PyResult<()> {
        self.builder = self.builder.clone().offset(offset);
        Ok(())
    }

    #[staticmethod]
    fn insert(table: &str) -> Self {
        Self {
            builder: QueryBuilder::insert(table),
        }
    }

    fn values(&mut self, values: Vec<String>) -> PyResult<()> {
        let vals: Vec<&str> = values.iter().map(|s| s.as_str()).collect();
        self.builder = self.builder.clone().values(vals);
        Ok(())
    }

    #[staticmethod]
    fn update(table: &str) -> Self {
        Self {
            builder: QueryBuilder::update(table),
        }
    }

    fn set(&mut self, column: &str, value: &str) -> PyResult<()> {
        self.builder = self.builder.clone().set(column, value);
        Ok(())
    }

    #[staticmethod]
    fn delete(table: &str) -> Self {
        Self {
            builder: QueryBuilder::delete(table),
        }
    }

    fn to_sql(&self) -> String {
        self.builder.to_sql()
    }
}

#[pyclass]
pub struct PyField {
    primary_key: bool,
    nullable: bool,
    default: Option<Py<PyAny>>,
}

#[pymethods]
impl PyField {
    #[new]
    #[pyo3(signature = (*, primary_key = false, nullable = false, default = None, unique = false))]
    fn new(
        primary_key: bool,
        nullable: bool,
        default: Option<Py<PyAny>>,
        unique: bool,
    ) -> Self {
        Self {
            primary_key,
            nullable,
            default,
        }
    }
}

/// Table decorator for model classes
/// 
/// This decorator allows users to configure table names and other model settings.
/// Example:
/// ```python
/// from rohas_orm import Table
/// 
/// @Table(name="users")
/// class User(Model):
///     id: int = Field(primary_key=True)
///     name: str
/// ```
#[pyclass]
pub struct PyTable {
    name: Option<String>,
    schema: Option<String>,
}

#[pymethods]
impl PyTable {
    #[new]
    #[pyo3(signature = (*, name = None, schema = None))]
    fn new(name: Option<String>, schema: Option<String>) -> Self {
        Self { name, schema }
    }

    fn __call__(
        &self,
        py: Python<'_>,
        cls: Py<PyAny>,
    ) -> PyResult<PyObject> {
        let cls_bound = cls.bind(py);
        
        if let Some(ref table_name) = self.name {
            cls_bound.setattr("__table_name__", table_name.as_str())?;
        }
        
        if let Some(ref schema_name) = self.schema {
            cls_bound.setattr("__table_schema__", schema_name.as_str())?;
        }
        
        Ok(cls.into())
    }
    
    fn get_name(&self) -> Option<String> {
        self.name.clone()
    }
    
    fn get_schema(&self) -> Option<String> {
        self.schema.clone()
    }
}

/// Index decorator for model fields
/// 
/// Example:
/// ```python
/// from rohas_orm import Index
/// 
/// @Index(fields=["email", "name"])
/// class User(Model):
///     email: str
///     name: str
/// ```
#[pyclass]
pub struct PyIndex {
    fields: Vec<String>,
    name: Option<String>,
    unique: bool,
}

#[pymethods]
impl PyIndex {
    #[new]
    #[pyo3(signature = (*, fields = None, name = None, unique = false))]
    fn new(
        fields: Option<Vec<String>>,
        name: Option<String>,
        unique: bool,
    ) -> Self {
        Self {
            fields: fields.unwrap_or_default(),
            name,
            unique,
        }
    }
    

    fn __call__(
        &self,
        py: Python<'_>,
        cls: Py<PyAny>,
    ) -> PyResult<PyObject> {
        let cls_bound = cls.bind(py);
        
        if !self.fields.is_empty() {
            let fields_list = PyList::empty(py);
            for field in &self.fields {
                fields_list.append(field)?;
            }
            cls_bound.setattr("__indexes__", fields_list.as_any())?;
        }
        
        if let Some(ref index_name) = self.name {
            cls_bound.setattr("__index_name__", index_name.as_str())?;
        }
        
        if self.unique {
            cls_bound.setattr("__index_unique__", true)?;
        }
        
        Ok(cls.into())
    }
    
    fn get_fields(&self) -> Vec<String> {
        self.fields.clone()
    }
    
    fn get_name(&self) -> Option<String> {
        self.name.clone()
    }
    
    fn is_unique(&self) -> bool {
        self.unique
    }
}

/// Unique constraint decorator for model fields
/// 
/// Example:
/// ```python
/// from rohas_orm import Unique
/// 
/// @Unique(fields=["email"])
/// class User(Model):
///     email: str
/// ```
#[pyclass]
pub struct PyUnique {
    fields: Vec<String>,
    name: Option<String>,
}

#[pymethods]
impl PyUnique {
    #[new]
    #[pyo3(signature = (*, fields = None, name = None))]
    fn new(
        fields: Option<Vec<String>>,
        name: Option<String>,
    ) -> Self {
        Self {
            fields: fields.unwrap_or_default(),
            name,
        }
    }

    fn __call__(
        &self,
        py: Python<'_>,
        cls: Py<PyAny>,
    ) -> PyResult<PyObject> {
        let cls_bound = cls.bind(py);
        
        if !self.fields.is_empty() {
            let fields_list = PyList::empty(py);
            for field in &self.fields {
                fields_list.append(field)?;
            }
            cls_bound.setattr("__unique_constraints__", fields_list.as_any())?;
        }
        
        if let Some(ref constraint_name) = self.name {
            cls_bound.setattr("__unique_name__", constraint_name.as_str())?;
        }
        
        Ok(cls.into())
    }
    
    
    fn get_fields(&self) -> Vec<String> {
        self.fields.clone()
    }
    
    fn get_name(&self) -> Option<String> {
        self.name.clone()
    }
}

#[pyfunction]
fn connect(url: &str) -> PyResult<PyDatabase> {
    PyDatabase::new(url)
}
