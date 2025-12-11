import asyncio
from generated.state import State
from generated.events.payment_failed import PaymentFailed


async def handle_payment_failure(event: PaymentFailed, state: State) -> None:
    """
    Handle payment failure - update order status and notify customer.
    """
    order_id = event.payload.get('orderId')
    reason = event.payload.get('reason', 'Unknown error')

    state.logger.warning(f'Payment failed for order {order_id}, reason: {reason}')

    await asyncio.sleep(0.1)

    # Update order status to failed
    state.trigger_event('OrderStatusUpdated', {
        'orderId': order_id,
        'status': 'payment_failed',
        'reason': reason
    })

    # Release inventory since payment failed
    state.trigger_event('InventoryReleased', {
        'orderId': order_id,
        'reason': 'payment_failed'
    })
