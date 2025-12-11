from generated.state import State
from generated.events.payment_refunded import PaymentRefunded


async def handle_update_order_status(event: PaymentRefunded, state: State) -> None:
    """Update order status after refund."""
    order_id = event.payload.get('orderId')

    state.logger.info(f'Updating order {order_id} status after refund')

    # In a real app, update order status in database
    state.trigger_event('OrderStatusUpdated', {
        'orderId': order_id,
        'status': 'refunded'
    })
