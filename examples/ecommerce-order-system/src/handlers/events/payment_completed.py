import asyncio
from generated.state import State
from generated.events.payment_processed import PaymentProcessed


async def handle_payment_completed(event: PaymentProcessed, state: State) -> None:
    """
    Mark payment as completed after successful processing.
    This triggers shipment creation.
    """
    order_id = event.payload.get('orderId')
    payment_id = event.payload.get('paymentId')

    state.logger.info(f'Payment {payment_id} completed for order {order_id}')

    await asyncio.sleep(0.1)  # 100ms - database update

    # Trigger PaymentCompleted event
    state.trigger_event('PaymentCompleted', {
        'orderId': order_id,
        'paymentId': payment_id,
        'amount': event.payload.get('amount', 0)
    })
