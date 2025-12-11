import asyncio
from generated.state import State
from generated.events.order_delivered import OrderDelivered


async def handle_complete_order(event: OrderDelivered, state: State) -> None:
    """
    Mark order as completed after delivery.
    """
    order_id = event.payload.get('orderId')

    state.logger.info(f'Completing order {order_id}')

    await asyncio.sleep(0.1)

    state.trigger_event('OrderStatusUpdated', {
        'orderId': order_id,
        'status': 'completed'
    })

    state.logger.info(f'Order {order_id} marked as completed')
