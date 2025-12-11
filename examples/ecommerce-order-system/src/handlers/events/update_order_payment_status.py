import asyncio
from generated.state import State
from generated.events.payment_processed import PaymentProcessed


async def handle_update_order_payment_status(event: PaymentProcessed, state: State) -> None:
    """
    Update order status after payment is processed.
    """
    order_id = event.payload.get('orderId')
    payment_id = event.payload.get('paymentId')

    state.logger.info(f'Updating order {order_id} payment status with payment {payment_id}')

    await asyncio.sleep(0.1)  # 100ms - database update

    # In a real app, update order in database
    state.trigger_event('OrderStatusUpdated', {
        'orderId': order_id,
        'status': 'paid',
        'paymentId': payment_id
    })
