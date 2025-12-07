"""
Utility functions for rohas-orm.
"""

from typing import Any, Dict, List


def dict_from_pydict(pydict) -> Dict[str, Any]:
    """
    Convert a PyDict to a regular Python dict.
    
    Args:
        pydict: PyDict object from PyO3
        
    Returns:
        Regular Python dictionary
    """
    result = {}
    for key in pydict.keys():
        result[key] = pydict[key]
    return result


def sanitize_sql_value(value: Any) -> str:
    """
    Sanitize a value for use in SQL queries.
    
    Args:
        value: Value to sanitize
        
    Returns:
        SQL-safe string representation
    """
    if value is None:
        return "NULL"
    elif isinstance(value, str):
        escaped = value.replace("'", "''")
        return f"'{escaped}'"
    elif isinstance(value, (int, float)):
        return str(value)
    elif isinstance(value, bool):
        return "1" if value else "0"
    else:
        escaped = str(value).replace("'", "''")
        return f"'{escaped}'"

