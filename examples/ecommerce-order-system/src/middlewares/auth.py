from typing import Dict, Any, Optional
from generated.state import State


async def auth_middleware(context: Dict[str, Any], state: State) -> Optional[Dict[str, Any]]:
    """
    Middleware for authentication.
    In a real app, validate JWT tokens or session cookies.
    """
    # For demo, we'll accept all requests
    # In production, validate authentication token

    api_name = context.get('api_name')
    if api_name:
        state.logger.debug(f'Auth middleware: {api_name}')

    # Pass through unchanged
    return None
