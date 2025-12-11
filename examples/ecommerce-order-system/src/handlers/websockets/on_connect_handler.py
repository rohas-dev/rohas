from generated.websockets.order_updates import OrderUpdatesConnection
from generated.state import State


async def on_connect_handler(connection: OrderUpdatesConnection, state: State) -> None:
    """Handle WebSocket connection for order updates."""
    state.logger.info(f'WebSocket connected: {connection.id}')

    # In a real app, you might store connection info for targeted updates
