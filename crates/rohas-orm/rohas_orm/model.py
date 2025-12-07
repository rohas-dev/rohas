"""
Model base class for rohas-orm.

This provides the base Model class that generated models inherit from.
"""

from typing import Optional, Dict, Any, List, TypeVar, Type
from rohas_orm import Database, QueryBuilder, Field
from .utils import dict_from_pydict, sanitize_sql_value

T = TypeVar('T', bound='Model')


class Model:
    """
    Base class for ORM models.
    
    Generated models should inherit from this class.
    """
    
    def __init__(self, **kwargs):
        """
        Initialize model instance with keyword arguments.
        
        Args:
            **kwargs: Field values to set on the instance
        """
        for key, value in kwargs.items():
            setattr(self, key, value)
    
    @classmethod
    def table_name(cls) -> str:
        """Get the table name for this model.
        
        Checks for __table_name__ attribute set by @Table decorator,
        otherwise defaults to lowercase class name + 's'.
        """
        # Check if table name was set by @Table decorator
        if hasattr(cls, '__table_name__'):
            return cls.__table_name__
        return f"{cls.__name__.lower()}s"
    
    @classmethod
    def primary_key(cls) -> str:
        """Get the primary key field name (defaults to 'id')."""
        # Check for fields with primary_key=True
        if hasattr(cls, '__annotations__'):
            for field_name, field_value in cls.__annotations__.items():
                if hasattr(cls, field_name):
                    field = getattr(cls, field_name)
                    if isinstance(field, Field) and hasattr(field, 'primary_key') and field.primary_key:
                        return field_name
        return "id"
    
    @classmethod
    def find_by_id(cls: Type[T], db: Database, id: int) -> Optional[T]:
        """
        Find a model by its primary key.
        
        Args:
            db: Database connection
            id: Primary key value
            
        Returns:
            Model instance or None if not found
        """
        query = QueryBuilder.select_all()
        query.from_(cls.table_name())
        query.where_eq_num(cls.primary_key(), id)
        query.limit(1)
        
        results = db.query(query)
        if not results or len(results) == 0:
            return None
        
        # Convert result dict to model instance
        # results is a list of PyDict objects
        row = results[0]
        data = dict_from_pydict(row)
        return cls.from_dict(data)
    
    @classmethod
    def find_all(cls: Type[T], db: Database) -> List[T]:
        """
        Find all models.
        
        Args:
            db: Database connection
            
        Returns:
            List of model instances
        """
        query = QueryBuilder.select_all()
        query.from_(cls.table_name())
        results = db.query(query)
        instances = []
        for row in results:
            data = dict_from_pydict(row)
            instances.append(cls.from_dict(data))
        return instances
    
    @classmethod
    def from_dict(cls: Type[T], data: Dict[str, Any]) -> T:
        """
        Create a model instance from a dictionary.
        
        Args:
            data: Dictionary with field values
            
        Returns:
            Model instance
        """
        instance = cls.__new__(cls)
        for key, value in data.items():
            setattr(instance, key, value)
        return instance
    
    def to_dict(self) -> Dict[str, Any]:
        """
        Convert model instance to dictionary.
        
        Returns:
            Dictionary with field values
        """
        result = {}
        if hasattr(self, '__annotations__'):
            for field_name in self.__annotations__.keys():
                if hasattr(self, field_name):
                    result[field_name] = getattr(self, field_name)
        return result
    
    def save(self, db: Database) -> None:
        """
        Save the model (insert or update).
        
        Args:
            db: Database connection
        """
        pk_field = self.primary_key()
        pk_value = getattr(self, pk_field, None)
        
        data = self.to_dict()
        
        existing = None
        if pk_value is not None:
            existing = self.__class__.find_by_id(db, pk_value)
        
        if existing:
            query = QueryBuilder.update(self.table_name())
            for key, value in data.items():
                if key != pk_field:
                    query.set(key, sanitize_sql_value(value))
            query.where_eq_num(pk_field, pk_value)
            db.execute(query.to_sql())
        else:
            columns = list(data.keys())
            values = [sanitize_sql_value(data[col]) for col in columns]
            
            query = QueryBuilder.insert(self.table_name())
            query.values(values)
            db.execute(query.to_sql())
    
    def delete(self, db: Database) -> None:
        """
        Delete the model from the database.
        
        Args:
            db: Database connection
        """
        pk_field = self.primary_key()
        pk_value = getattr(self, pk_field)
        
        query = QueryBuilder.delete(self.table_name())
        query.where_eq_num(pk_field, pk_value)
        
        db.execute(query.to_sql())
    
    @classmethod
    def create(cls: Type[T], db: Database, **kwargs) -> T:
        """
        Create a new model instance and save it.
        
        Args:
            db: Database connection
            **kwargs: Field values
            
        Returns:
            Created model instance
        """
        instance = cls(**kwargs)
        instance.save(db)
        return instance
    
    def __repr__(self) -> str:
        """String representation of the model."""
        fields = ", ".join(f"{k}={v!r}" for k, v in self.to_dict().items())
        return f"{self.__class__.__name__}({fields})"

