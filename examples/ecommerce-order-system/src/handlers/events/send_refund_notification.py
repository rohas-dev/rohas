import asyncio
from generated.state import State
from generated.events.payment_refunded import PaymentRefunded


async def handle_send_refund_notification(event: PaymentRefunded, state: State) -> None:
    """Send refund notification to customer."""
    order_id = event.payload.get('orderId')
    refund_id = event.payload.get('refundId')
    amount = event.payload.get('amount', 0)

    state.logger.info(f'Sending refund notification for order {order_id}, refund {refund_id}, amount: ${amount:.2f}')

    await asyncio.sleep(0.1)
    # In a real app, send email/SMS notification
