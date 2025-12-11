from generated.state import State
from generated.events.order_shipped import OrderShipped


async def handle_update_tracking(event: OrderShipped, state: State) -> None:
    """Update tracking information in database."""
    order_id = event.payload.get('orderId')
    tracking_number = event.payload.get('trackingNumber')

    state.logger.info(f'Updating tracking for order {order_id}: {tracking_number}')

    # In a real app, update order record with tracking info
