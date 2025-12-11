from typing import Optional
from generated.websockets.order_updates import OrderUpdatesMessage, OrderUpdatesConnection
from generated.state import State


async def on_message_handler(
    message: OrderUpdatesMessage,
    connection: OrderUpdatesConnection,
    state: State
) -> Optional[dict]:
    """
    Handle WebSocket messages for order updates.
    Supports subscribe/unsubscribe to order updates.
    """
    msg_type = message.data.type if message.data else None
    order_id = message.data.orderId if message.data else None

    state.logger.info(f'WebSocket message received: type={msg_type}, orderId={order_id}')

    if msg_type == "subscribe":
        # In a real app, subscribe connection to order updates
        return {
            "type": "subscribed",
            "orderId": order_id,
            "message": f"Subscribed to updates for order {order_id}"
        }
    elif msg_type == "unsubscribe":
        # In a real app, unsubscribe connection from order updates
        return {
            "type": "unsubscribed",
            "orderId": order_id,
            "message": f"Unsubscribed from updates for order {order_id}"
        }
    else:
        return {
            "type": "error",
            "message": "Unknown message type"
        }
