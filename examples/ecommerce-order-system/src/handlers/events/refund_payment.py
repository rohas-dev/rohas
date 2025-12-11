import asyncio
from generated.state import State
from generated.events.order_cancelled import OrderCancelled


async def handle_refund_payment(event: OrderCancelled, state: State) -> None:
    """
    Process refund when order is cancelled.
    """
    order_id = event.payload.get('orderId')
    total = event.payload.get('total', 0)

    state.logger.info(f'Processing refund for order {order_id}, amount: ${total:.2f}')

    # Simulate refund processing
    await asyncio.sleep(0.6)  # 600ms - payment gateway refund

    refund_id = f"refund_{order_id}_{state.generate_id()}"

    state.logger.info(f'Refund {refund_id} processed for order {order_id}')

    state.trigger_event('PaymentRefunded', {
        'orderId': order_id,
        'refundId': refund_id,
        'amount': total
    })
