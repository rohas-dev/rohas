import asyncio
from generated.state import State
from generated.events.order_created import OrderCreated


async def handle_process_payment(event: OrderCreated, state: State) -> None:
    """
    Process payment for the order.
    This is triggered by OrderCreated event.
    """
    order_id = event.payload.get('orderId')
    total = event.payload.get('total', 0)

    state.logger.info(f'Processing payment for order {order_id}, amount: ${total:.2f}')

    # Simulate payment processing time
    await asyncio.sleep(0.5)  # 500ms - payment gateway call

    # Simulate payment processing
    payment_id = f"pay_{order_id}_{state.generate_id()}"

    # In a real app, call payment gateway API
    # For demo, we'll simulate success
    payment_success = True

    if payment_success:
        state.logger.info(f'Payment {payment_id} processed successfully')
        state.trigger_event('PaymentProcessed', {
            'orderId': order_id,
            'paymentId': payment_id,
            'amount': total,
            'status': 'processing'
        })
    else:
        state.logger.error(f'Payment failed for order {order_id}')
        state.trigger_event('PaymentFailed', {
            'orderId': order_id,
            'paymentId': payment_id,
            'amount': total,
            'reason': 'Insufficient funds'
        })
