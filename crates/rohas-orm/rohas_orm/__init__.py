"""
rohas_orm - A modern ORM with Rust macros, Python annotations, codegen, and query builder.

This package is a Python extension module built with PyO3.
The Python bindings are defined in src/python.rs and compiled to a native extension.
"""

__version__ = "0.1.0"

try:
    import sys
    import importlib.util
    import os
    
    Database = None
    QueryBuilder = None
    Field = None
    Table = None
    Index = None
    Unique = None
    connect = None
    
    try:
        from rohas_orm import rohas_orm as _extension
        Database = getattr(_extension, 'PyDatabase', None)
        QueryBuilder = getattr(_extension, 'PyQueryBuilder', None)
        Field = getattr(_extension, 'PyField', None)
        Table = getattr(_extension, 'PyTable', None)
        Index = getattr(_extension, 'PyIndex', None)
        Unique = getattr(_extension, 'PyUnique', None)
        connect = getattr(_extension, 'connect', None)
        
        if Database is None or QueryBuilder is None or Field is None or connect is None:
            available = [x for x in dir(_extension) if not x.startswith('_')]
            raise ImportError(
                f"Extension module imported but missing required components.\n"
                f"Database: {Database is not None}, QueryBuilder: {QueryBuilder is not None}, "
                f"Field: {Field is not None}, connect: {connect is not None}\n"
                f"Available in extension: {available}"
            )
    except ImportError as e:
        try:
            import importlib
            _extension = importlib.import_module('rohas_orm')
            if hasattr(_extension, 'PyDatabase'):
                Database = _extension.PyDatabase
                QueryBuilder = _extension.PyQueryBuilder
                Field = _extension.PyField
                Table = getattr(_extension, 'PyTable', None)
                Index = getattr(_extension, 'PyIndex', None)
                Unique = getattr(_extension, 'PyUnique', None)
                connect = _extension.connect
            else:
                raise e
        except Exception:
            if 'rohas_orm.rohas_orm' in sys.modules:
                try:
                    _ext_mod = sys.modules['rohas_orm.rohas_orm']
                    if hasattr(_ext_mod, 'PyDatabase'):
                        Database = _ext_mod.PyDatabase
                        QueryBuilder = _ext_mod.PyQueryBuilder
                        Field = _ext_mod.PyField
                        Table = getattr(_ext_mod, 'PyTable', None)
                        Index = getattr(_ext_mod, 'PyIndex', None)
                        Unique = getattr(_ext_mod, 'PyUnique', None)
                        connect = _ext_mod.connect
                except Exception:
                    pass
    except Exception as e:
        import warnings
        warnings.warn(f"Failed to load rohas_orm extension: {e}", ImportWarning)
        pass
    
    if Database is None or QueryBuilder is None or Field is None or connect is None:
        current_dir = os.path.dirname(os.path.abspath(__file__))
        so_path = os.path.join(current_dir, "rohas_orm.abi3.so")
        
        error_details = []
        error_details.append(f"Database: {Database is not None}")
        error_details.append(f"QueryBuilder: {QueryBuilder is not None}")
        error_details.append(f"Field: {Field is not None}")
        error_details.append(f"connect: {connect is not None}")
        error_details.append(f"Extension .so exists: {os.path.exists(so_path) if so_path else False}")
        
        if 'rohas_orm.rohas_orm' in sys.modules:
            error_details.append("Extension module found in sys.modules")
        else:
            error_details.append("Extension module NOT in sys.modules")
        
        raise ImportError(
            f"rohas_orm extension module not available.\n"
            f"Details: {', '.join(error_details)}\n\n"
            f"The extension module needs to be built and installed.\n\n"
            f"For development:\n"
            f"  cd crates/rohas-orm\n"
            f"  maturin develop\n\n"
            f"Or install via pip:\n"
            f"  pip install rohas-orm"
        )
    
    if Table is None:
        def _raise_table_error(*args, **kwargs):
            raise ImportError("Table decorator requires rohas_orm extension. Please rebuild the package.")
        
        class _TableStub:
            def __init__(self, **kwargs):
                _raise_table_error()
            def __call__(self, cls):
                _raise_table_error()
        Table = _TableStub
    
    if Index is None:
        def _raise_index_error(*args, **kwargs):
            raise ImportError("Index decorator requires rohas_orm extension. Please rebuild the package.")
        
        class _IndexStub:
            def __init__(self, **kwargs):
                _raise_index_error()
            def __call__(self, cls):
                _raise_index_error()
        Index = _IndexStub
    
    if Unique is None:
        def _raise_unique_error(*args, **kwargs):
            raise ImportError("Unique decorator requires rohas_orm extension. Please rebuild the package.")
        
        class _UniqueStub:
            def __init__(self, **kwargs):
                _raise_unique_error()
            def __call__(self, cls):
                _raise_unique_error()
        Unique = _UniqueStub
    
    from .model import Model
    
    __all__ = [
        "Database",
        "QueryBuilder",
        "Field",
        "Table",
        "Index",
        "Unique",
        "Model",
        "connect",
        "__version__",
    ]
except (ImportError, AttributeError, OSError, FileNotFoundError) as e:
    __all__ = ["__version__"]
    
    _error_msg = (
        "rohas_orm extension module not found.\n"
        "The Python bindings are compiled from Rust code in src/python.rs.\n\n"
        "To install:\n"
        "  pip install rohas-orm\n\n"
        "For development:\n"
        "  cd crates/rohas-orm\n"
        "  maturin develop"
    )
    
    def _raise_error():
        raise ImportError(_error_msg)
    
    class Database:
        """Database connection - requires compiled extension"""
        def __init__(self, url: str):
            _raise_error()
    
    class QueryBuilder:
        """Query builder - requires compiled extension"""
        def __init__(self):
            _raise_error()
    
    class Field:
        """Field annotation - requires compiled extension"""
        def __init__(self, **kwargs):
            _raise_error()
    
    class Table:
        """Table decorator - requires compiled extension"""
        def __init__(self, **kwargs):
            _raise_error()
    
    class Index:
        """Index decorator - requires compiled extension"""
        def __init__(self, **kwargs):
            _raise_error()
    
    class Unique:
        """Unique constraint decorator - requires compiled extension"""
        def __init__(self, **kwargs):
            _raise_error()
    
    class Model:
        """Model base class - requires compiled extension"""
        def __init__(self, **kwargs):
            _raise_error()
    
    def connect(url: str):
        """Connect to database - requires compiled extension"""
        _raise_error()

