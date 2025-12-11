from generated.websockets.order_updates import OrderUpdatesConnection
from generated.state import State


async def on_disconnect_handler(connection: OrderUpdatesConnection, state: State) -> None:
    """Handle WebSocket disconnection."""
    state.logger.info(f'WebSocket disconnected: {connection.id}')

    # In a real app, clean up connection subscriptions
