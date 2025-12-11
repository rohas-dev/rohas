from typing import Dict, Any, Optional
from generated.state import State


async def request_logging_middleware(context: Dict[str, Any], state: State) -> Optional[Dict[str, Any]]:
    """
    Middleware for request logging.
    Logs all incoming requests for monitoring and debugging.
    """
    api_name = context.get('api_name')
    trace_id = context.get('trace_id')

    if api_name:
        state.logger.info(f'Request: {api_name}, trace_id: {trace_id}')

    # Pass through unchanged
    return None
