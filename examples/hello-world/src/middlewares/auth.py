from typing import Dict, Any, Optional
from generated.state import State

async def auth_middleware(context: Dict[str, Any], state: State) -> Optional[Dict[str, Any]]:
    """
    Middleware function for auth.

    Args:
        context: Request context containing:
            - payload: Request payload (for APIs)
            - query_params: Query parameters (for APIs)
            - connection: WebSocket connection info (for WebSockets)
            - websocket_name: WebSocket name (for WebSockets)
            - api_name: API name (for APIs)
            - trace_id: Trace ID
        state: State object for logging and triggering events

    Returns:
        Optional[Dict[str, Any]]: Modified context with 'payload' and/or 'query_params' keys,
        or None to pass through unchanged. Return a dict with 'error' key to reject the request.

    To reject the request, raise an exception 
    """
    # TODO: Implement middleware logic
    # Example: Validate authentication
    # Example: Rate limiting
    # Example: Logging
    # Example: Modify payload/query_params
    # 
    # To modify the request:
    # return {
    #     'payload': modified_payload,
    #     'query_params': modified_query_params
    # }
    # 
    # To reject the request:
    # raise Exception('Access denied')
    
    # Pass through unchanged
    return None
