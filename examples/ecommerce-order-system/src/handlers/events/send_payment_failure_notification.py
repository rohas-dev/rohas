import asyncio
from generated.state import State
from generated.events.payment_failed import PaymentFailed


async def handle_send_payment_failure_notification(event: PaymentFailed, state: State) -> None:
    """Send payment failure notification to customer."""
    order_id = event.payload.get('orderId')
    reason = event.payload.get('reason', 'Unknown error')

    state.logger.info(f'Sending payment failure notification for order {order_id}, reason: {reason}')

    await asyncio.sleep(0.1)
    # In a real app, send email/SMS notification
